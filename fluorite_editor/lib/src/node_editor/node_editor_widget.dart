import 'package:flutter/material.dart';


import 'models.dart';
import 'serialization.dart';
import '../ui/streaming_profiler_overlay.dart';

class NodeEditorScreen extends StatefulWidget {
  const NodeEditorScreen({super.key});

  @override
  State<NodeEditorScreen> createState() => _NodeEditorScreenState();
}

class _NodeEditorScreenState extends State<NodeEditorScreen> {
  final NodeGraph graph = NodeGraph();
  GraphNode? selectedNode;

  // Connection Dragging State
  Offset? _dragPos;
  String? _dragFromNodeId;
  String? _dragFromPinId;

  @override
  void initState() {
    super.initState();
    // Add some initial nodes
    final emitter = NodeFactory.createComputeParticleEmitter(
      'node_1',
      const Offset(50, 100),
    );
    final vfx = NodeFactory.createHalfPrecisionVFX(
      'node_2',
      const Offset(400, 100),
    );
    final pcg = NodeFactory.createMobileOptimizedTerrainPCG(
      'node_3',
      const Offset(50, 300),
    );

    graph.addNode(emitter);
    graph.addNode(vfx);
    graph.addNode(pcg);

    graph.addConnection(
      NodeConnection(
        fromNodeId: 'node_1',
        fromPinId: 'out_particles',
        toNodeId: 'node_2',
        toPinId: 'in_particles',
      ),
    );
  }

  void _addNode(String type) {
    setState(() {
      final id = 'node_${DateTime.now().millisecondsSinceEpoch}';
      final pos = const Offset(100, 100);
      switch (type) {
        case 'ComputeParticleEmitter':
          graph.addNode(NodeFactory.createComputeParticleEmitter(id, pos));
          break;
        case 'MobileOptimizedTerrainPCG':
          graph.addNode(NodeFactory.createMobileOptimizedTerrainPCG(id, pos));
          break;
        case 'HalfPrecisionVFX':
          graph.addNode(NodeFactory.createHalfPrecisionVFX(id, pos));
          break;
      }
    });
  }

  void _showAddNodeMenu(BuildContext context) {
    showModalBottomSheet(
      context: context,
      builder: (context) {
        return ListView(
          shrinkWrap: true,
          children: [
            ListTile(
              title: const Text('Add Compute Particle Emitter'),
              onTap: () {
                _addNode('ComputeParticleEmitter');
                Navigator.pop(context);
              },
            ),
            ListTile(
              title: const Text('Add Mobile Optimized Terrain PCG'),
              onTap: () {
                _addNode('MobileOptimizedTerrainPCG');
                Navigator.pop(context);
              },
            ),
            ListTile(
              title: const Text('Add Half Precision VFX'),
              onTap: () {
                _addNode('HalfPrecisionVFX');
                Navigator.pop(context);
              },
            ),
          ],
        );
      },
    );
  }

  void _exportGraph() {
    final jsonStr = GraphSerialization.toJson(graph);
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Exported Graph JSON'),
        content: SingleChildScrollView(child: SelectableText(jsonStr)),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }

  void _deleteSelectedNode() {
    if (selectedNode != null) {
      setState(() {
        graph.removeNode(selectedNode!.id);
        selectedNode = null;
      });
    }
  }

  void _deleteConnection(NodeConnection connection) {
    setState(() {
      graph.removeConnection(connection.fromNodeId, connection.fromPinId,
          connection.toNodeId, connection.toPinId);
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Node Editor'),
        actions: [
          if (selectedNode != null)
            IconButton(
              icon: const Icon(Icons.delete),
              onPressed: _deleteSelectedNode,
              tooltip: 'Delete Selected Node',
            ),
          IconButton(
            icon: const Icon(Icons.add),
            onPressed: () => _showAddNodeMenu(context),
          ),
          IconButton(
            icon: const Icon(Icons.save),
            onPressed: _exportGraph,
          ),
        ],
      ),
      body: Row(
        children: [
          Expanded(
            child: GestureDetector(
              onTap: () {
                setState(() {
                  selectedNode = null;
                });
              },
              onPanUpdate: (details) {
                if (_dragFromNodeId != null) {
                  setState(() {
                    _dragPos = details.localPosition;
                  });
                }
              },
              onPanEnd: (details) {
                setState(() {
                  _dragFromNodeId = null;
                  _dragFromPinId = null;
                  _dragPos = null;
                });
              },
              child: Stack(
                children: [
                  // Draw connections
                  CustomPaint(
                    size: Size.infinite,
                    painter:
                        _ConnectionPainter(graph, _dragFromNodeId, _dragPos),
                  ),
                  // Render invisible hitboxes for deleting connections (middle of spline)
                  for (final connection in graph.connections)
                    Builder(builder: (context) {
                      try {
                        final fromNode = graph.nodes
                            .firstWhere((n) => n.id == connection.fromNodeId);
                        final toNode = graph.nodes
                            .firstWhere((n) => n.id == connection.toNodeId);
                        final p1 = fromNode.position + const Offset(250, 40);
                        final p2 = toNode.position + const Offset(0, 40);
                        // approximate midpoint of cubic bezier
                        final midX = p1.dx + (p2.dx - p1.dx) / 2;
                        final midY = p1.dy + (p2.dy - p1.dy) / 2;
                        return Positioned(
                            left: midX - 12,
                            top: midY - 12,
                            child: IconButton(
                              icon: const Icon(Icons.close,
                                  size: 16, color: Colors.redAccent),
                              onPressed: () => _deleteConnection(connection),
                              tooltip: 'Delete Connection',
                            ));
                      } catch (_) {
                        return const SizedBox.shrink();
                      }
                    }),
                  // Draw nodes
                  for (final node in graph.nodes)
                    Positioned(
                      left: node.position.dx,
                      top: node.position.dy,
                      child: GestureDetector(
                        onPanUpdate: (details) {
                          setState(() {
                            node.position += details.delta;
                          });
                        },
                        onTap: () {
                          setState(() {
                            selectedNode = node;
                          });
                        },
                        child: _NodeWidget(
                          node: node,
                          isSelected: selectedNode == node,
                          onPinDragStart: (pinId, isInput, localPos) {
                            if (!isInput) {
                              setState(() {
                                _dragFromNodeId = node.id;
                                _dragFromPinId = pinId;
                                _dragPos = node.position + localPos;
                              });
                            }
                          },
                          onPinDrop: (pinId, isInput) {
                            if (isInput &&
                                _dragFromNodeId != null &&
                                _dragFromPinId != null &&
                                _dragFromNodeId != node.id) {
                              setState(() {
                                graph.addConnection(NodeConnection(
                                  fromNodeId: _dragFromNodeId!,
                                  fromPinId: _dragFromPinId!,
                                  toNodeId: node.id,
                                  toPinId: pinId,
                                ));
                                _dragFromNodeId = null;
                                _dragFromPinId = null;
                                _dragPos = null;
                              });
                            }
                          },
                        ),
                      ),
                    ),
                  const StreamingProfilerOverlay(),
                ],
              ),
            ),
          ),
          if (selectedNode != null)
            Container(
              width: 300,
              color: Colors.black26,
              child: _PropertiesPanel(
                node: selectedNode!,
                onChanged: () => setState(() {}),
              ),
            ),
        ],
      ),
    );
  }
}

class _NodeWidget extends StatelessWidget {
  final GraphNode node;
  final bool isSelected;
  final Function(String pinId, bool isInput, Offset localPos)? onPinDragStart;
  final Function(String pinId, bool isInput)? onPinDrop;

  const _NodeWidget({
    required this.node,
    required this.isSelected,
    this.onPinDragStart,
    this.onPinDrop,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 280,
      decoration: BoxDecoration(
        color: const Color(0xFF2D2D30),
        border: Border.all(
          color: isSelected ? Colors.blueAccent : Colors.black,
          width: 2,
        ),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Container(
            padding: const EdgeInsets.all(8),
            decoration: const BoxDecoration(
              color: Colors.black54,
              borderRadius: BorderRadius.vertical(top: Radius.circular(6)),
            ),
            child: Text(
              node.name,
              style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 14),
            ),
          ),
          Padding(
            padding: const EdgeInsets.all(8.0),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: node.inputs.map((pin) {
                    return _PinWidget(
                      pin: pin,
                      isInput: true,
                      onDragStart: (pos) =>
                          onPinDragStart?.call(pin.id, true, pos),
                      onDrop: () => onPinDrop?.call(pin.id, true),
                    );
                  }).toList(),
                ),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.end,
                  children: node.outputs.map((pin) {
                    return _PinWidget(
                      pin: pin,
                      isInput: false,
                      onDragStart: (pos) =>
                          onPinDragStart?.call(pin.id, false, pos),
                      onDrop: () => onPinDrop?.call(pin.id, false),
                    );
                  }).toList(),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _PinWidget extends StatelessWidget {
  final NodePin pin;
  final bool isInput;
  final Function(Offset)? onDragStart;
  final VoidCallback? onDrop;

  const _PinWidget(
      {required this.pin,
      required this.isInput,
      this.onDragStart,
      this.onDrop});

  @override
  Widget build(BuildContext context) {
    Widget pinUi = Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (isInput) const Icon(Icons.circle, size: 12, color: Colors.blueGrey),
        if (isInput) const SizedBox(width: 4),
        Text(pin.name, style: const TextStyle(fontSize: 12)),
        if (!isInput) const SizedBox(width: 4),
        if (!isInput)
          const Icon(Icons.circle, size: 12, color: Colors.blueGrey),
      ],
    );

    return MouseRegion(
      cursor: SystemMouseCursors.click,
      child: GestureDetector(
        onPanStart: (details) {
          onDragStart?.call(details.localPosition);
        },
        onPanUpdate: (details) {
          // Needed to capture pan but NodeEditorScreen handles overall drawing
        },
        child: DragTarget<String>(
          onAcceptWithDetails: (details) => onDrop?.call(),
          builder: (context, candidateData, rejectedData) {
            // For simplicity relying on the parent's HitTest logic, but wrapping allows future extensions
            return Container(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: Draggable<String>(
                data: pin.id,
                feedback: Container(
                    width: 12,
                    height: 12,
                    decoration: const BoxDecoration(
                        shape: BoxShape.circle, color: Colors.blueAccent)),
                childWhenDragging: Opacity(opacity: 0.5, child: pinUi),
                onDragStarted: () {
                  final box = context.findRenderObject() as RenderBox?;
                  if (box != null) {
                    // Approximating local pos for line start
                    onDragStart?.call(Offset(isInput ? 0 : 50, 10));
                  }
                },
                onDragEnd: (details) {
                  // Cleanup handled by parent
                },
                child: MouseRegion(
                    onEnter: (_) {
                      // Hover effect logic
                    },
                    child: pinUi),
              ),
            );
          },
        ),
      ),
    );
  }
}

class _ConnectionPainter extends CustomPainter {
  final NodeGraph graph;
  final String? dragFromNodeId;
  final Offset? dragPos;

  _ConnectionPainter(this.graph, this.dragFromNodeId, this.dragPos);

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.white54
      ..strokeWidth = 2
      ..style = PaintingStyle.stroke;

    for (final connection in graph.connections) {
      try {
        final fromNode = graph.nodes.firstWhere(
          (n) => n.id == connection.fromNodeId,
        );
        final toNode = graph.nodes.firstWhere(
          (n) => n.id == connection.toNodeId,
        );

        // Better estimation of pin position
        final p1 = fromNode.position + const Offset(250, 40);
        final p2 = toNode.position + const Offset(0, 40);

        final path = Path();
        path.moveTo(p1.dx, p1.dy);
        path.cubicTo(p1.dx + 50, p1.dy, p2.dx - 50, p2.dy, p2.dx, p2.dy);
        canvas.drawPath(path, paint);
      } catch (_) {}
    }

    if (dragFromNodeId != null && dragPos != null) {
      final fromNode = graph.nodes.firstWhere((n) => n.id == dragFromNodeId,
          orElse: () => graph.nodes.first);
      final p1 = fromNode.position + const Offset(250, 40);

      final dragPaint = Paint()
        ..color = Colors.blueAccent
        ..strokeWidth = 2
        ..style = PaintingStyle.stroke;

      final path = Path();
      path.moveTo(p1.dx, p1.dy);
      path.cubicTo(p1.dx + 50, p1.dy, dragPos!.dx - 50, dragPos!.dy,
          dragPos!.dx, dragPos!.dy);
      canvas.drawPath(path, dragPaint);
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => true;
}

class _PropertiesPanel extends StatelessWidget {
  final GraphNode node;
  final VoidCallback onChanged;

  const _PropertiesPanel({required this.node, required this.onChanged});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(16.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Properties: ${node.name}',
            style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 18),
          ),
          const Divider(),
          ...node.properties.entries.map((entry) {
            return Padding(
              padding: const EdgeInsets.symmetric(vertical: 8.0),
              child: Row(
                children: [
                  Expanded(child: Text(entry.key)),
                  Expanded(
                    child: TextFormField(
                      key: ValueKey("${node.id}_${entry.key}"),
                      initialValue: entry.value.toString(),
                      onChanged: (val) {
                        if (entry.value is double) {
                          node.properties[entry.key] =
                              double.tryParse(val) ?? entry.value;
                        } else if (entry.value is bool) {
                          node.properties[entry.key] =
                              val.toLowerCase() == 'true';
                        } else {
                          node.properties[entry.key] = val;
                        }
                        onChanged();
                      },
                    ),
                  ),
                ],
              ),
            );
          }),
        ],
      ),
    );
  }
}




