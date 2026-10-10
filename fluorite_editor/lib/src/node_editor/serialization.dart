import 'dart:convert';
import 'package:flutter/material.dart';
import 'models.dart';

class GraphSerialization {
  static String toJson(NodeGraph graph) {
    final Map<String, dynamic> data = {
      'nodes': graph.nodes.map((node) {
        final Map<String, dynamic> baseData = {
          'id': node.id,
          'name': node.name,
          'type': node.type,
          'position': {'x': node.position.dx, 'y': node.position.dy},
          'properties': node.properties,
        };

        if (node.type == 'MobileOptimizedTerrainPCG') {
          final biomeScale =
              (node.properties['biome_scale'] as num?)?.toDouble() ?? 1.0;
          final density =
              (node.properties['density'] as num?)?.toDouble() ?? 1.0;
          final seed = (node.properties['seed'] as num?)?.toInt() ?? 0;

          baseData['engine_payload'] = {
            'config': {
              'biome_scale': biomeScale,
              'density': density,
            },
            'seed': seed,
          };
        } else if (node.type == 'HalfPrecisionVFX') {
          final maxParticles =
              (node.properties['max_particles'] as num?)?.toInt() ?? 1000;
          final emitRate =
              (node.properties['emit_rate'] as num?)?.toDouble() ?? 100.0;

          baseData['engine_payload'] = {
            'max_particles': maxParticles,
            'emit_rate': emitRate,
          };
        }

        return baseData;
      }).toList(),
      'connections': graph.connections
          .map((conn) => {
                'from': conn.fromNodeId,
                'from_pin': conn.fromPinId,
                'to': conn.toNodeId,
                'to_pin': conn.toPinId,
              })
          .toList(),
    };
    return jsonEncode(data);
  }

  static void fromJson(String jsonStr, NodeGraph graph) {
    final data = jsonDecode(jsonStr);

    graph.nodes.clear();
    graph.connections.clear();

    if (data['nodes'] != null) {
      for (var nodeData in data['nodes']) {
        final id = nodeData['id'];
        final type = nodeData['type'];
        final pos = Offset(
          (nodeData['position']['x'] as num).toDouble(),
          (nodeData['position']['y'] as num).toDouble(),
        );

        GraphNode node;
        switch (type) {
          case 'ComputeParticleEmitter':
            node = NodeFactory.createComputeParticleEmitter(id, pos);
            break;
          case 'MobileOptimizedTerrainPCG':
            node = NodeFactory.createMobileOptimizedTerrainPCG(id, pos);
            break;
          case 'HalfPrecisionVFX':
            node = NodeFactory.createHalfPrecisionVFX(id, pos);
            break;
          default:
            // Fallback for unknown nodes
            node = GraphNode(
              id: id,
              name: nodeData['name'] ?? 'Unknown',
              type: type,
              position: pos,
              properties:
                  Map<String, dynamic>.from(nodeData['properties'] ?? {}),
            );
        }

        // Overwrite properties if they exist
        if (nodeData['properties'] != null) {
          node.properties = Map<String, dynamic>.from(nodeData['properties']);
        }

        switch (type) {
          case 'MobileOptimizedTerrainPCG':
            if (nodeData['engine_payload'] != null) {
              final payload = nodeData['engine_payload'];
              if (payload['config'] != null) {
                node.properties['biome_scale'] =
                    (payload['config']['biome_scale'] as num?)?.toDouble() ??
                        1.0;
                node.properties['density'] =
                    (payload['config']['density'] as num?)?.toDouble() ?? 1.0;
              }
              node.properties['seed'] = (payload['seed'] as num?)?.toInt() ?? 0;
            }
            break;
          case 'HalfPrecisionVFX':
            if (nodeData['engine_payload'] != null) {
              final payload = nodeData['engine_payload'];
              node.properties['max_particles'] =
                  (payload['max_particles'] as num?)?.toInt() ?? 1000;
              node.properties['emit_rate'] =
                  (payload['emit_rate'] as num?)?.toDouble() ?? 100.0;
            }
            break;
        }

        graph.addNode(node);
      }
    }

    if (data['connections'] != null) {
      for (var connData in data['connections']) {
        graph.addConnection(NodeConnection(
          fromNodeId: connData['from'],
          fromPinId: connData['from_pin'],
          toNodeId: connData['to'],
          toPinId: connData['to_pin'],
        ));
      }
    }
  }
}
