# Fluoridian App Architecture Report

## Overview
`fluoridian_app` is a Flutter-based decentralized application (DApp) and protocol client operating within the TitheX ecosystem. It serves as the primary user-facing interface for interacting with the decentralized network, governance, and mesh capabilities.

## Tech Stack
- **Frontend Framework**: Flutter (Dart)
- **State Management**: Riverpod (`flutter_riverpod`)
- **Native Bridge**: `flutter_rust_bridge` (v2)
- **Rust Backend**: Uses a local Rust crate `rust_lib_fluoridian_app` which in turn depends on `rust_core`.

## Core Features & Screens
The application acts as a central hub with the following key modules:

1. **Staking (`staking_screen.dart`)**: Interfaces for locking tokens, tracking yields, and managing validator stakes.
2. **Subscription (`subscription_screen.dart`)**: Management of decentralized subscriptions (likely for mesh access or premium services).
3. **Governance (`governance_screen.dart`)**: Voting mechanisms and DAO participation.
4. **Treasury (`treasury_screen.dart`)**: Financial overview of the protocol's treasury and fund allocation.
5. **Marketplace (`marketplace_screen.dart`)**: Decentralized exchange or asset trading platform.
6. **SOCKS5 Mesh (`socks5_screen.dart`)**: Control panel for the decentralized networking layer, likely interacting directly with the Rust core to establish secure tunneling/routing.
7. **Task Board (`board_page.dart` / `task_verification_page.dart`)**: Decentralized task management and verification (Kanban style).
8. **Constellation Map (`constellation_map_page.dart`)**: Visual representation of the node network or satellite-style mesh network telemetry.

## System Architecture

```mermaid
flowchart TD
    A[Flutter App UI] --> B[Riverpod State]
    B --> C[flutter_rust_bridge (FFI)]
    C --> D[rust_lib_fluoridian_app]
    D --> E[TitheX rust_core]
    
    subgraph UI Modules
    UI1[Staking/Treasury]
    UI2[SOCKS5 Mesh Control]
    UI3[Constellation Map]
    end
    
    A -.-> UI1
    A -.-> UI2
    A -.-> UI3
```

## Integration Points for Fluorescent
While Fluorescent is a 3D AAA rendering bridge, `fluoridian_app` represents the decentralized control interface. 
If Fluorescent plans to integrate with Fluoridian, the likely connection points are:
1. **Telemetry & Visualization**: The `constellation_map_page.dart` could be upgraded using Fluorescent's 3D engine to render the node mesh in a fully 3D interactive view.
2. **Shared Rust Core**: `fluoridian_app` connects to `rust_core` via FFI. Fluorescent's `fluorite_core` can tap into this same shared state to visualize network activity or utilize the SOCKS5 mesh for multiplayer networking (`fluoderpod`).
3. **Audio Capture**: `fluoridian_app` includes `just_audio` and `record` packages, which could feed into Fluorescent's real-time acoustics and spatial audio pipelines.
