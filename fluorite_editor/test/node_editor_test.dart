import 'package:flutter_test/flutter_test.dart';
import 'package:fluorite_editor/src/node_editor/models.dart';
import 'package:fluorite_editor/src/node_editor/serialization.dart';

void main() {
  group('NodeEditor Models and Serialization', () {
    test('Graph Serialization and Deserialization', () {
      final graph = NodeGraph();

      final emitter =
          NodeFactory.createComputeParticleEmitter('n1', const Offset(10, 20));
      emitter.properties['spawn_rate'] = 500.0; // Change a property
      graph.addNode(emitter);

      final vfx =
          NodeFactory.createHalfPrecisionVFX('n2', const Offset(100, 200));
      graph.addNode(vfx);

      graph.addConnection(NodeConnection(
        fromNodeId: 'n1',
        fromPinId: 'out_particles',
        toNodeId: 'n2',
        toPinId: 'in_particles',
      ));

      final jsonStr = GraphSerialization.toJson(graph);
      expect(jsonStr, isNotEmpty);
      expect(jsonStr.contains('ComputeParticleEmitter'), isTrue);

      final newGraph = NodeGraph();
      GraphSerialization.fromJson(jsonStr, newGraph);

      expect(newGraph.nodes.length, 2);
      expect(newGraph.connections.length, 1);

      final n1 = newGraph.nodes.firstWhere((n) => n.id == 'n1');
      expect(n1.type, 'ComputeParticleEmitter');
      expect(n1.position.dx, 10);
      expect(n1.properties['spawn_rate'], 500.0);

      final c1 = newGraph.connections.first;
      expect(c1.fromNodeId, 'n1');
      expect(c1.toNodeId, 'n2');
    });
  });
}
