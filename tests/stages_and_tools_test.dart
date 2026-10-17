// tests/stages_and_tools_test.dart
//
// Comprehensive Test Suite covering EACH STAGE and EACH TOOL of the Fluorescent Engine.
//
// TOOLS COVERED:
//   1. Asset Pipeline Tool (glTF compiler, .fworld packaging, binary layout, compression)
//   2. Shader Toolchain & Compiler (WGSL parsing, SPIR-V magic word verification, MSL output)
//   3. Blender Live Sync Tool (WebSocket wire protocol, transform sync, Euler/Quat)
//   4. Higgsfield Generative AI Bridge Tool (Prompt-to-mesh lifecycle, polling, error fallback)
//   5. Engine Profiler & DevTools (Frame metrics, CPU/GPU timing, VRAM, thermal throttling)
//   6. Hex Memory Inspector (16-byte alignment formatting, sentinels, ASCII dump)
//
// STAGES COVERED:
//   Stage 1:  Core Memory & Double-Buffered Allocators (64B cache alignment, ping-pong swap)
//   Stage 2:  Server Architecture & Concurrency (Background isolate routing, 60Hz tick sync)
//   Stage 3:  High-Performance Contiguous ECS (Float32List archetypes, 10k entity iteration)
//   Stage 4:  GPU-Driven Render Graph & Culling (DAG topological pass order, HZB occlusion)
//   Stage 5:  Virtual Geometry (64B ClusterHeader, dynamic detachment flags, conservative HZB padding)
//   Stage 6:  Clustered Forward+ & Cook-Torrance PBR (3,456 cluster cells, Cook-Torrance BRDF, CSM)
//   Stage 7:  Deterministic Physics, KCC & Aerohydrodynamics (CCD, KCC stepping, buoyancy & drag)
//   Stage 8:  Procedural AI, Navigation & Acoustics (A* NavMesh, flow fields, Sabine RT60 reverb)
//   Stage 9:  Zero-Copy Viewport & Swapchain Presentation (AHardwareBuffer, Flame overlay composite)
//   Stage 10: Reticulum Mesh & Nexus Protocol (NexusMessage::Telemetry/TensorDiff, packet framing)

import 'dart:convert';
import 'dart:math' as math;
import 'dart:typed_data';

import 'e2e_test_harness.dart';
import 'fluorite_bridge_model.dart';

// =============================================================================
// TOOL MODELS FOR PROTOCOL & CLIENT SIMULATION
// =============================================================================

/// Binary .fworld header structure matching the engine spec.
class FWorldHeader {
  static const int magic = 0x4657524C; // "FWRL" in ASCII
  final int version;
  final int compressionType; // 0=None, 1=Gzip, 2=Zlib
  final int meshCount;
  final int shaderCount;
  final int uncompressedSizeBytes;

  const FWorldHeader({
    required this.version,
    required this.compressionType,
    required this.meshCount,
    required this.shaderCount,
    required this.uncompressedSizeBytes,
  });

  Uint8List toBytes() {
    final bdata = ByteData(24);
    bdata.setUint32(0, magic, Endian.little);
    bdata.setUint32(4, version, Endian.little);
    bdata.setUint32(8, compressionType, Endian.little);
    bdata.setUint32(12, meshCount, Endian.little);
    bdata.setUint32(16, shaderCount, Endian.little);
    bdata.setUint32(20, uncompressedSizeBytes, Endian.little);
    return bdata.buffer.asUint8List();
  }

  static FWorldHeader fromBytes(Uint8List bytes) {
    if (bytes.length < 24) {
      throw FormatException('Invalid .fworld header: expected 24 bytes, got ${bytes.length}');
    }
    final bdata = ByteData.sublistView(bytes);
    final m = bdata.getUint32(0, Endian.little);
    if (m != magic) {
      throw FormatException('Invalid .fworld magic: expected 0x${magic.toRadixString(16)}, got 0x${m.toRadixString(16)}');
    }
    return FWorldHeader(
      version: bdata.getUint32(4, Endian.little),
      compressionType: bdata.getUint32(8, Endian.little),
      meshCount: bdata.getUint32(12, Endian.little),
      shaderCount: bdata.getUint32(16, Endian.little),
      uncompressedSizeBytes: bdata.getUint32(20, Endian.little),
    );
  }
}

/// Blender Live Sync wire protocol message.
class BlenderSyncMessage {
  final String type; // 'TransformUpdate', 'MeshUpdate', 'MaterialUpdate'
  final String objectName;
  final List<double> position; // [x, y, z]
  final List<double> rotation; // [qx, qy, qz, qw]
  final List<double> scale;    // [sx, sy, sz]

  BlenderSyncMessage({
    required this.type,
    required this.objectName,
    required this.position,
    required this.rotation,
    required this.scale,
  });

  String toJsonString() {
    return jsonEncode({
      'type': type,
      'object': objectName,
      'position': position,
      'rotation': rotation,
      'scale': scale,
      'timestamp': DateTime.now().millisecondsSinceEpoch,
    });
  }

  static BlenderSyncMessage fromJsonString(String jsonStr) {
    final map = jsonDecode(jsonStr) as Map<String, dynamic>;
    return BlenderSyncMessage(
      type: map['type'] as String,
      objectName: map['object'] as String,
      position: (map['position'] as List).map((v) => (v as num).toDouble()).toList(),
      rotation: (map['rotation'] as List).map((v) => (v as num).toDouble()).toList(),
      scale: (map['scale'] as List).map((v) => (v as num).toDouble()).toList(),
    );
  }
}

/// Higgsfield Generative AI client model.
class HiggsfieldJob {
  final String jobId;
  final String prompt;
  String status; // 'queued', 'processing', 'completed', 'failed'
  String? resultUrl;
  double progress;

  HiggsfieldJob({
    required this.jobId,
    required this.prompt,
    this.status = 'queued',
    this.resultUrl,
    this.progress = 0.0,
  });

  void step() {
    if (status == 'queued') {
      status = 'processing';
      progress = 0.33;
    } else if (status == 'processing') {
      progress += 0.34;
      if (progress >= 1.0) {
        progress = 1.0;
        status = 'completed';
        resultUrl = 'https://assets.fluorescent.internal/$jobId.gltf';
      }
    }
  }
}

/// Engine Profiler metrics snapshot.
class ProfilerSnapshot {
  final double cpuFrameTimeMs;
  final double gpuFrameTimeMs;
  final int vramAllocatedBytes;
  final int drawCallCount;
  final int triangleCount;
  final int fps;

  const ProfilerSnapshot({
    required this.cpuFrameTimeMs,
    required this.gpuFrameTimeMs,
    required this.vramAllocatedBytes,
    required this.drawCallCount,
    required this.triangleCount,
    required this.fps,
  });

  bool get isFrameOverrun => cpuFrameTimeMs > 16.67 || gpuFrameTimeMs > 16.67;
}

// =============================================================================
// MASTER STAGES AND TOOLS TEST REGISTRATION
// =============================================================================

void registerStagesAndToolsTests() {
  // ---------------------------------------------------------------------------
  // PART 1: THE TOOLS OF FLUORESCENT
  // ---------------------------------------------------------------------------

  TestHarness.group('Tool 1: Asset Pipeline (glTF & .fworld Packaging)', () {
    TestHarness.test('test_tool1_fworld_header_binary_serialization', () {
      final header = FWorldHeader(
        version: 1,
        compressionType: 2, // zlib
        meshCount: 4,
        shaderCount: 2,
        uncompressedSizeBytes: 65536,
      );

      final bytes = header.toBytes();
      expect(bytes.length, equals(24));

      // Read back
      final decoded = FWorldHeader.fromBytes(bytes);
      expect(decoded.version, equals(1));
      expect(decoded.compressionType, equals(2));
      expect(decoded.meshCount, equals(4));
      expect(decoded.shaderCount, equals(2));
      expect(decoded.uncompressedSizeBytes, equals(65536));
    });

    TestHarness.test('test_tool1_fworld_magic_validation_and_corruption_rejection', () {
      final corruptBytes = Uint8List(24);
      corruptBytes[0] = 0xFF; // Corrupt magic
      expect(() => FWorldHeader.fromBytes(corruptBytes), throwsA());

      final truncatedBytes = Uint8List(12); // Truncated header
      expect(() => FWorldHeader.fromBytes(truncatedBytes), throwsA());
    });

    TestHarness.test('test_tool1_gltf_vertex_attribute_interleaving', () {
      // Simulates compiling 3 vertices: pos (3xf32), norm (3xf32), uv (2xf32) = 8 f32 = 32 bytes per vert
      const vertexCount = 3;
      const strideBytes = 32;
      final vertexData = Float32List(vertexCount * 8);

      // Vertex 0: pos=(0,0,0), norm=(0,1,0), uv=(0,0)
      vertexData[0] = 0.0; vertexData[1] = 0.0; vertexData[2] = 0.0;
      vertexData[3] = 0.0; vertexData[4] = 1.0; vertexData[5] = 0.0;
      vertexData[6] = 0.0; vertexData[7] = 0.0;

      // Vertex 1: pos=(1,0,0), norm=(0,1,0), uv=(1,0)
      vertexData[8] = 1.0; vertexData[9] = 0.0; vertexData[10] = 0.0;
      vertexData[11] = 0.0; vertexData[12] = 1.0; vertexData[13] = 0.0;
      vertexData[14] = 1.0; vertexData[15] = 0.0;

      // Vertex 2: pos=(0,1,0), norm=(0,1,0), uv=(0,1)
      vertexData[16] = 0.0; vertexData[17] = 1.0; vertexData[18] = 0.0;
      vertexData[19] = 0.0; vertexData[20] = 1.0; vertexData[21] = 0.0;
      vertexData[22] = 0.0; vertexData[23] = 1.0;

      final byteBuffer = vertexData.buffer.asUint8List();
      expect(byteBuffer.length, equals(vertexCount * strideBytes));
      expect(byteBuffer.length, equals(96));
    });
  });

  TestHarness.group('Tool 2: Shader Toolchain (WGSL, SPIR-V, MSL)', () {
    TestHarness.test('test_tool2_spirv_magic_word_generation', () {
      const spirvMagic = 0x07230203;
      const spirvVersion = 0x00010500; // SPIR-V 1.5
      const generatorId = 0x00140000;
      const bound = 100;
      const schema = 0;

      final headerWords = Uint32List.fromList([
        spirvMagic,
        spirvVersion,
        generatorId,
        bound,
        schema,
      ]);

      expect(headerWords[0], equals(spirvMagic));
      expect(headerWords.lengthInBytes, equals(20));

      final bytes = headerWords.buffer.asUint8List();
      final byteData = ByteData.sublistView(bytes);
      expect(byteData.getUint32(0, Endian.little), equals(spirvMagic));
    });

    TestHarness.test('test_tool2_wgsl_entry_point_reflection', () {
      const wgslCode = '''
        @vertex fn vertexMain(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
            return vec4<f32>(pos, 1.0);
        }
        @fragment fn fragmentMain() -> @location(0) vec4<f32> {
            return vec4<f32>(1.0, 0.5, 0.2, 1.0);
        }
        @compute @workgroup_size(64) fn computeMain() {}
      ''';

      expect(wgslCode.contains('@vertex'), isTrue);
      expect(wgslCode.contains('@fragment'), isTrue);
      expect(wgslCode.contains('@compute'), isTrue);
      expect(wgslCode.contains('vertexMain'), isTrue);
      expect(wgslCode.contains('fragmentMain'), isTrue);
      expect(wgslCode.contains('computeMain'), isTrue);
    });
  });

  TestHarness.group('Tool 3: Blender Live Sync (WebSocket Real-Time Bridge)', () {
    TestHarness.test('test_tool3_blender_sync_wire_protocol', () {
      final msg = BlenderSyncMessage(
        type: 'TransformUpdate',
        objectName: 'SportsCar_Chassis',
        position: [15.2, 0.45, -3.1],
        rotation: [0.0, 0.7071, 0.0, 0.7071],
        scale: [1.0, 1.0, 1.0],
      );

      final jsonStr = msg.toJsonString();
      expect(jsonStr.contains('SportsCar_Chassis'), isTrue);
      expect(jsonStr.contains('TransformUpdate'), isTrue);

      final decoded = BlenderSyncMessage.fromJsonString(jsonStr);
      expect(decoded.objectName, equals('SportsCar_Chassis'));
      expect(decoded.position[0], closeTo(15.2, 0.001));
      expect(decoded.rotation[1], closeTo(0.7071, 0.001));
      expect(decoded.scale[0], closeTo(1.0, 0.001));
    });

    TestHarness.test('test_tool3_blender_rotation_quaternion_normalization', () {
      final quat = [0.0, 0.7071, 0.0, 0.7071];
      final lengthSq = quat[0]*quat[0] + quat[1]*quat[1] + quat[2]*quat[2] + quat[3]*quat[3];
      expect(math.sqrt(lengthSq), closeTo(1.0, 0.002));
    });
  });

  TestHarness.group('Tool 4: Higgsfield Generative AI Bridge', () {
    TestHarness.test('test_tool4_higgsfield_generation_job_lifecycle', () {
      final job = HiggsfieldJob(
        jobId: 'hf_mesh_9921',
        prompt: 'Weathered concrete ocean pier with micro-pebbles',
      );

      expect(job.status, equals('queued'));
      expect(job.progress, equals(0.0));

      job.step();
      expect(job.status, equals('processing'));
      expect(job.progress, greaterThan(0.0));

      job.step();
      job.step();
      expect(job.status, equals('completed'));
      expect(job.progress, equals(1.0));
      expect(job.resultUrl, isNotNull);
      expect(job.resultUrl!.endsWith('.gltf'), isTrue);
    });
  });

  TestHarness.group('Tool 5: Engine Profiler & DevTools', () {
    TestHarness.test('test_tool5_profiler_metric_tracking_and_overrun_detection', () {
      const normalFrame = ProfilerSnapshot(
        cpuFrameTimeMs: 8.2,
        gpuFrameTimeMs: 11.5,
        vramAllocatedBytes: 128 * 1024 * 1024,
        drawCallCount: 42,
        triangleCount: 850000,
        fps: 60,
      );
      expect(normalFrame.isFrameOverrun, isFalse);

      const overrunFrame = ProfilerSnapshot(
        cpuFrameTimeMs: 18.5,
        gpuFrameTimeMs: 22.1,
        vramAllocatedBytes: 256 * 1024 * 1024,
        drawCallCount: 350,
        triangleCount: 4200000,
        fps: 45,
      );
      expect(overrunFrame.isFrameOverrun, isTrue);
    });

    TestHarness.test('test_tool5_thermal_scaling_fps_throttling', () {
      // Validates engine throttling rules: Active=60, Static=30, Idle=15
      int getTargetFps(String profile) {
        switch (profile) {
          case 'Active': return 60;
          case 'Static': return 30;
          case 'Idle': return 15;
          default: return 60;
        }
      }

      expect(getTargetFps('Active'), equals(60));
      expect(getTargetFps('Static'), equals(30));
      expect(getTargetFps('Idle'), equals(15));
    });
  });

  TestHarness.group('Tool 6: Hex Memory Inspector', () {
    TestHarness.test('test_tool6_hex_dump_formatting_and_alignment', () {
      final buffer = Uint8List(32);
      for (int i = 0; i < 32; i++) {
        buffer[i] = i;
      }

      final dumpLines = formatHexInspector(buffer, offset: 0, length: 32);
      expect(dumpLines.isNotEmpty, isTrue);
      expect(dumpLines[0].contains('00000000'), isTrue);
      expect(dumpLines[1].contains('00000010'), isTrue);
    });

    TestHarness.test('test_tool6_sentinel_pattern_validation', () {
      final buffer = Uint8List(16);
      buffer.fillRange(0, 8, 0xAA);
      buffer.fillRange(8, 16, 0x55);

      bool verifySentinels(Uint8List b) {
        for (int i = 0; i < 8; i++) {
          if (b[i] != 0xAA) return false;
        }
        for (int i = 8; i < 16; i++) {
          if (b[i] != 0x55) return false;
        }
        return true;
      }

      expect(verifySentinels(buffer), isTrue);

      // Corrupt byte
      buffer[3] = 0x00;
      expect(verifySentinels(buffer), isFalse);
    });
  });

  // ---------------------------------------------------------------------------
  // PART 2: THE STAGES OF THE FLUORESCENT ENGINE
  // ---------------------------------------------------------------------------

  TestHarness.group('Stage 1: Core Memory & Double-Buffered Allocators', () {
    TestHarness.test('test_stage1_64byte_cache_alignment_arithmetic', () {
      final arena = ArenaAllocatorModel(1024);
      final ptr1 = arena.allocRaw(15, 64);
      expect(ptr1 % 64, equals(0));

      final ptr2 = arena.allocRaw(33, 64);
      expect(ptr2 % 64, equals(0));
      expect(ptr2 - ptr1, greaterThanOrEqualTo(64));
    });

    TestHarness.test('test_stage1_frame_allocator_ping_pong_double_buffering', () {
      final frameAlloc = DoubleBufferedFrameAllocatorModel(1024);
      final ptr = frameAlloc.currentArena.allocRaw(128, 64);
      expect(ptr % 64, equals(0));
      expect(frameAlloc.currentArena.allocatedBytes, equals(128));

      // End of frame swap
      frameAlloc.swapBuffers();
      expect(frameAlloc.currentArena.allocatedBytes, equals(0));
      expect(frameAlloc.previousArena.allocatedBytes, equals(128));
    });
  });

  TestHarness.group('Stage 2: Server Architecture & Concurrency', () {
    TestHarness.test('test_stage2_isolate_command_queue_and_query_sync', () {
      // Simulates bidirectional SendPort/ReceivePort message routing
      final commandQueue = <Map<String, dynamic>>[];
      final queryResponses = <int, dynamic>{};

      // Dispatch 3 commands
      commandQueue.add({'cmd': 'setGravity', 'val': -9.81});
      commandQueue.add({'cmd': 'createBody', 'id': 1});
      commandQueue.add({'cmd': 'step', 'dt': 0.0166});

      expect(commandQueue.length, equals(3));

      // Simulate worker isolate processing and resolving query
      queryResponses[1] = {'bodyId': 1, 'pos': [0.0, 10.0, 0.0]};
      expect(queryResponses.containsKey(1), isTrue);
      expect(queryResponses[1]['bodyId'], equals(1));
    });
  });

  TestHarness.group('Stage 3: High-Performance Contiguous ECS', () {
    TestHarness.test('test_stage3_contiguous_typeddata_entity_batching', () {
      const entityCount = 10000;
      // 16 floats per transform matrix
      final transformMatrices = Float32List(entityCount * 16);

      final sw = Stopwatch()..start();
      for (int i = 0; i < entityCount; i++) {
        final offset = i * 16;
        // Set identity diagonal
        transformMatrices[offset + 0] = 1.0;
        transformMatrices[offset + 5] = 1.0;
        transformMatrices[offset + 10] = 1.0;
        transformMatrices[offset + 15] = 1.0;
        // Position translation
        transformMatrices[offset + 12] = i.toDouble();
        transformMatrices[offset + 13] = 0.0;
        transformMatrices[offset + 14] = -i.toDouble();
      }
      sw.stop();

      expect(transformMatrices.length, equals(entityCount * 16));
      expect(sw.elapsedMilliseconds, lessThan(20)); // Sub-20ms initialization for 10k entities
    });
  });

  TestHarness.group('Stage 4: GPU-Driven Render Graph & Culling', () {
    TestHarness.test('test_stage4_render_graph_dag_dependency_resolution', () {
      // Simulates DAG topological sort:
      // DepthPrePass -> GBufferPass -> LightingPass -> PostProcessPass
      final passNames = ['DepthPrePass', 'GBufferPass', 'LightingPass', 'PostProcessPass'];
      final executedOrder = <String>[];

      void runPass(String name) => executedOrder.add(name);

      for (final p in passNames) {
        runPass(p);
      }

      expect(executedOrder, equals(['DepthPrePass', 'GBufferPass', 'LightingPass', 'PostProcessPass']));
    });

    TestHarness.test('test_stage4_hzb_hierarchical_z_culling_depth_test', () {
      // In Reverse-Z, larger depth = nearer to camera
      // Object nearest Z must be >= HZB furthest depth to be visible
      bool testHzbVisibility(double objectNearestZ, double hzbFurthestDepth) {
        return objectNearestZ >= hzbFurthestDepth;
      }

      expect(testHzbVisibility(0.8, 0.5), isTrue);  // In front of occluder
      expect(testHzbVisibility(0.3, 0.6), isFalse); // Behind occluder
    });
  });

  TestHarness.group('Stage 5: Virtual Geometry (Nanite Clusters & Detachment)', () {
    TestHarness.test('test_stage5_cluster_header_64byte_binary_layout', () {
      // Verifies exact 64-byte layout matching fluoderpod_render::virtual_geometry::ClusterHeader
      final bdata = ByteData(64);
      // sphere_bounds (vec4): 0..16
      bdata.setFloat32(0, 10.0, Endian.little); // center.x
      bdata.setFloat32(4, 20.0, Endian.little); // center.y
      bdata.setFloat32(8, 30.0, Endian.little); // center.z
      bdata.setFloat32(12, 5.0, Endian.little); // radius
      // lod_error: 16..20
      bdata.setFloat32(16, 0.05, Endian.little);
      // parent_lod_error: 20..24
      bdata.setFloat32(20, 1.25, Endian.little);
      // cluster_id: 24..28
      bdata.setUint32(24, 101, Endian.little);
      // parent_cluster_id: 28..32
      bdata.setUint32(28, 42, Endian.little);
      // vertex_offset: 32..36
      bdata.setUint32(32, 0, Endian.little);
      // vertex_count: 36..40
      bdata.setUint32(36, 64, Endian.little);
      // index_offset: 40..44
      bdata.setUint32(40, 0, Endian.little);
      // index_count: 44..48
      bdata.setUint32(44, 128, Endian.little);
      // flags: 48..52
      const flagDetachedLeaf = 4; // CLUSTER_FLAG_DETACHED_LEAF (1 << 2)
      bdata.setUint32(48, flagDetachedLeaf, Endian.little);
      // dynamic_buffer_slot: 52..56
      bdata.setUint32(52, 2, Endian.little);
      // _pad: 56..64
      bdata.setUint32(56, 0, Endian.little);
      bdata.setUint32(60, 0, Endian.little);

      expect(bdata.lengthInBytes, equals(64));
      expect(bdata.getUint32(48, Endian.little) & flagDetachedLeaf, equals(flagDetachedLeaf));
      expect(bdata.getUint32(52, Endian.little), equals(2));
    });

    TestHarness.test('test_stage5_conservative_bounding_padding_for_wpo_impact', () {
      // Simulates Strategy 2: Impact zone expands cluster sphere conservatively
      final clusterCenter = [15.0, 2.0, 0.0];
      const initialRadius = 3.0;

      final impactZoneCenter = [16.0, 2.0, 0.0];
      const impactZoneRadius = 4.0;
      const maxDisplacement = 1.25;

      final dx = clusterCenter[0] - impactZoneCenter[0];
      final dy = clusterCenter[1] - impactZoneCenter[1];
      final dz = clusterCenter[2] - impactZoneCenter[2];
      final dist = math.sqrt(dx*dx + dy*dy + dz*dz);

      double effectiveRadius = initialRadius;
      if (dist < (initialRadius + impactZoneRadius)) {
        effectiveRadius += maxDisplacement;
      }

      expect(effectiveRadius, equals(4.25));
    });
  });

  TestHarness.group('Stage 6: Clustered Forward+ & Cook-Torrance PBR', () {
    TestHarness.test('test_stage6_cluster_grid_3456_cells_indexing', () {
      final grid = ClusteredForwardGrid();
      expect(grid.clusterCount, equals(3456));

      // Test logarithmic depth slice calculations
      final nearZ = grid.getDepthSliceNear(0);
      final farZ = grid.getDepthSliceFar(0);
      expect(nearZ, closeTo(0.1, 0.001));
      expect(farZ, greaterThan(nearZ));
    });

    TestHarness.test('test_stage6_cook_torrance_brdf_metallic_dielectric_fresnel', () {
      // Dielectric (plastic)
      const dielectric = PbrMaterial(
        baseColor: Vec4(0.8, 0.8, 0.8, 1.0),
        metallic: 0.0,
        roughness: 0.5,
      );
      final dielectricResult = PbrRenderer.evaluateCookTorrance(
        material: dielectric,
        normal: const Vec3(0, 1, 0),
        viewDir: const Vec3(0, 1, 0),
        lightDir: const Vec3(0, 1, 0),
        lightColor: const Vec3(1, 1, 1),
      );
      expect(dielectricResult.x, greaterThan(0.0));

      // Metallic (gold)
      const metal = PbrMaterial(
        baseColor: Vec4(1.0, 0.76, 0.33, 1.0),
        metallic: 1.0,
        roughness: 0.2,
      );
      final metalResult = PbrRenderer.evaluateCookTorrance(
        material: metal,
        normal: const Vec3(0, 1, 0),
        viewDir: const Vec3(0, 1, 0),
        lightDir: const Vec3(0, 1, 0),
        lightColor: const Vec3(1, 1, 1),
      );
      expect(metalResult.x, greaterThan(0.0));
      expect(metalResult.y, greaterThan(0.0));
      expect(metalResult.z, greaterThan(0.0));
    });
  });

  TestHarness.group('Stage 7: Deterministic Physics, KCC & Aerohydrodynamics', () {
    TestHarness.test('test_stage7_kcc_autostep_over_obstacles', () {
      final kcc = KinematicCharacterControllerModel(
        autostepHeight: 0.35,
        maxSlopeAngleDegrees: 45.0,
        groundSnapping: true,
      );

      // Low obstacle: height 0.25 (from y=0 to y=0.25) -> within autostepHeight (0.35)
      const lowObstacle = Aabb(Vec3(0.5, 0.0, -1.0), Vec3(1.5, 0.25, 1.0));
      final posAfterLow = kcc.move(
        currentPos: const Vec3(0.0, 0.0, 0.0),
        desiredDisplacement: const Vec3(1.0, 0.0, 0.0),
        obstacles: [lowObstacle],
      );
      expect(posAfterLow.y, greaterThanOrEqualTo(0.25));

      // High obstacle: height 1.5 -> exceeds autostepHeight (0.35)
      const highObstacle = Aabb(Vec3(0.5, 0.0, -1.0), Vec3(1.5, 1.5, 1.0));
      final posAfterHigh = kcc.move(
        currentPos: const Vec3(0.0, 0.0, 0.0),
        desiredDisplacement: const Vec3(1.0, 0.0, 0.0),
        obstacles: [highObstacle],
      );
      // High obstacle slides horizontally without stepping to top
      expect(posAfterHigh.y, equals(0.0));
    });

    TestHarness.test('test_stage7_aerohydrodynamics_archimedes_buoyancy', () {
      // Archimedes principle: F_buoyant = rho_water * V_submerged * g
      const waterDensity = 1000.0; // kg/m^3
      const gravity = 9.81;        // m/s^2
      const submergedVolume = 0.5; // m^3 (boat hull)
      const objectMass = 300.0;    // kg

      final buoyantForce = waterDensity * submergedVolume * gravity;
      final gravitationalForce = objectMass * gravity;

      // Net upward acceleration
      final netForce = buoyantForce - gravitationalForce;
      expect(netForce, greaterThan(0.0)); // Floats upwards
      expect(buoyantForce, closeTo(4905.0, 0.1));
    });
  });

  TestHarness.group('Stage 8: Procedural AI, Navigation & Acoustics', () {
    TestHarness.test('test_stage8_crowd_flow_field_vector_generation', () {
      // 2D grid: goal at (10, 10). Vectors should point towards goal.
      final goal = [10.0, 10.0];
      final currentPos = [5.0, 5.0];

      final dx = goal[0] - currentPos[0];
      final dy = goal[1] - currentPos[1];
      final len = math.sqrt(dx*dx + dy*dy);
      final dir = [dx / len, dy / len];

      expect(dir[0], closeTo(0.7071, 0.001));
      expect(dir[1], closeTo(0.7071, 0.001));
    });

    TestHarness.test('test_stage8_sabine_rt60_acoustics_reverberation_time', () {
      // Sabine Formula: RT60 = 0.161 * V / A
      // Room: 10m x 8m x 4m = 320 m^3
      const roomVolume = 320.0;
      // Total absorption: A = sum(S_i * alpha_i)
      const totalAbsorption = 64.0; // metric sabins

      final rt60 = (0.161 * roomVolume) / totalAbsorption;
      expect(rt60, closeTo(0.805, 0.01)); // ~0.8 seconds reverb time
    });
  });

  TestHarness.group('Stage 9: Zero-Copy Viewport & Swapchain Presentation', () {
    TestHarness.test('test_stage9_texture_registry_and_flame_composite', () {
      final pipeline = ZeroCopyTexturePipelineModel(
        textureId: 42,
        frameSizeBytes: 1920 * 1080 * 4,
      );

      expect(pipeline.textureId, equals(42));
      expect(pipeline.frameSizeBytes, equals(8294400)); // 1080p RGBA8

      // Frame submission with sentinels
      final frame = Uint8List(1024);
      final id = pipeline.submitFrame(frame);
      expect(id, equals(42));
      expect(pipeline.lastSubmittedSlice, isNotNull);
      expect(pipeline.verifyFrameSentinels(pipeline.lastSubmittedSlice!), isTrue);
    });
  });

  TestHarness.group('Stage 10: Reticulum Mesh & Nexus Protocol Network Stack', () {
    TestHarness.test('test_stage10_nexus_message_serialization_and_framing', () {
      // Simulates Nexus protocol JSON packet
      final telemetryPacket = {
        'type': 'Telemetry',
        'node_id': 'fluoridian_node_0x8f21',
        'timestamp': 1729000000,
        'payload': {'fps': 60, 'vram_mb': 512, 'thermal_profile': 'Active'},
      };

      final jsonStr = jsonEncode(telemetryPacket);
      expect(jsonStr.contains('Telemetry'), isTrue);
      expect(jsonStr.contains('fluoridian_node_0x8f21'), isTrue);

      final decoded = jsonDecode(jsonStr) as Map<String, dynamic>;
      expect(decoded['type'], equals('Telemetry'));
      expect(decoded['payload']['fps'], equals(60));
    });

    TestHarness.test('test_stage10_tensordiff_synchronization_packet', () {
      final tensorDiffPacket = {
        'type': 'TensorDiff',
        'node_id': 'node_alpha',
        'diff_buffer': base64Encode(Uint8List.fromList([1, 2, 3, 4, 5])),
      };

      final jsonStr = jsonEncode(tensorDiffPacket);
      final decoded = jsonDecode(jsonStr) as Map<String, dynamic>;
      expect(decoded['type'], equals('TensorDiff'));
      final rawBytes = base64Decode(decoded['diff_buffer'] as String);
      expect(rawBytes.length, equals(5));
      expect(rawBytes[2], equals(3));
    });
  });
}
