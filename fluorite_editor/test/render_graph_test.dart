import 'package:flutter_test/flutter_test.dart';
import 'package:fluorite_editor/src/node_editor/models.dart';
import 'package:fluorite_editor/src/node_editor/serialization.dart';
import 'dart:convert';

void main() {
  test('Serialize to Render Graph', () {
    final graph = NodeGraph();
    final pcg = NodeFactory.createMobileOptimizedTerrainPCG('pcg1', Offset.zero);
    pcg.properties['biome_scale'] = 2.5;
    graph.addNode(pcg);

    final vfx = NodeFactory.createHalfPrecisionVFX('vfx1', Offset.zero);
    vfx.properties['emit_rate'] = 500.0;
    graph.addNode(vfx);

    final jsonStr = GraphSerialization.toJson(graph);
    final data = jsonDecode(jsonStr);

    expect(data['nodes'][0]['engine_payload']['config']['biome_scale'], 2.5);
    expect(data['nodes'][1]['engine_payload']['emit_rate'], 500.0);
  });
}
