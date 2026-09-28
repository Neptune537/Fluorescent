import 'package:flutter/widgets.dart';

/// Lore entry in the Fluorite Codex.
class CodexEntry {
  final String id;
  final String title;
  final String faction;
  final String classification;
  final String excerpt;
  final bool isUnlocked;

  const CodexEntry({
    required this.id,
    required this.title,
    required this.faction,
    required this.classification,
    required this.excerpt,
    required this.isUnlocked,
  });
}

/// Mission Command Terminal and Episodic Briefing Deck UI.
class NarrativeDeckView extends StatefulWidget {
  final List<CodexEntry> codexEntries;
  final String activeMissionName;
  final String activeMissionObjective;
  final VoidCallback? onClose;

  const NarrativeDeckView({
    super.key,
    required this.codexEntries,
    required this.activeMissionName,
    required this.activeMissionObjective,
    this.onClose,
  });

  @override
  State<NarrativeDeckView> createState() => _NarrativeDeckViewState();
}

class _NarrativeDeckViewState extends State<NarrativeDeckView> {
  int _selectedEntryIndex = 0;

  @override
  Widget build(BuildContext context) {
    return Container(
      color: const Color(0xF5060D1A),
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header Bar
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Column(

                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'TACTICAL NARRATIVE DECK',
                      style: TextStyle(
                        color: Color(0xFF00E5FF),
                        fontSize: 20,
                        fontWeight: FontWeight.w900,
                        letterSpacing: 2.0,
                      ),
                    ),
                    Text(
                      'TRANSMISSION NODE: FLUORITE CORE // EPISODIC BRIEFING',
                      style: TextStyle(
                        color: const Color(0xFF00E5FF).withAlpha(160),
                        fontSize: 10,
                        letterSpacing: 1.2,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 16),

              GestureDetector(
                onTap: widget.onClose,
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
                  decoration: BoxDecoration(
                    border: Border.all(color: const Color(0xFFFF5252)),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: const Text(
                    'CLOSE TERMINAL [ESC]',
                    style: TextStyle(color: Color(0xFFFF5252), fontSize: 11, fontWeight: FontWeight.bold),
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),

          // Active Mission Briefing Strip
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: const Color(0x3300E5FF),
              border: Border.all(color: const Color(0xFF00E5FF)),
              borderRadius: BorderRadius.circular(4),
            ),
            child: Row(
              children: [
                const Icon(
                  IconData(0xe0b3, fontFamily: 'MaterialIcons'),
                  color: Color(0xFF00E5FF),
                  size: 20,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'PRIORITY DIRECTIVE: ${widget.activeMissionName.toUpperCase()}',
                        style: const TextStyle(color: Color(0xFF00E5FF), fontSize: 12, fontWeight: FontWeight.bold),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        widget.activeMissionObjective,
                        style: const TextStyle(color: Color(0xFFE0F7FA), fontSize: 11),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),

          // Split View: Codex List on left, details on right
          Expanded(
            child: Row(
              children: [
                // Left: Codex entries list
                Expanded(
                  flex: 2,
                  child: Container(
                    decoration: BoxDecoration(
                      border: Border.all(color: const Color(0x3300E5FF)),
                    ),
                    child: ListView.builder(
                      itemCount: widget.codexEntries.length,
                      itemBuilder: (context, index) {
                        final entry = widget.codexEntries[index];
                        final isSelected = index == _selectedEntryIndex;
                        return GestureDetector(
                          onTap: () => setState(() => _selectedEntryIndex = index),
                          child: Container(
                            padding: const EdgeInsets.all(12),
                            color: isSelected ? const Color(0x3300E5FF) : const Color(0x00000000),
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(
                                  entry.title,
                                  style: TextStyle(
                                    color: isSelected ? const Color(0xFF00E5FF) : const Color(0xDDFFFFFF),
                                    fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
                                    fontSize: 13,
                                  ),
                                ),
                                const SizedBox(height: 2),
                                Text(
                                  '${entry.faction} // ${entry.classification}',
                                  style: const TextStyle(color: Color(0x77FFFFFF), fontSize: 10),
                                ),
                              ],
                            ),
                          ),
                        );
                      },
                    ),
                  ),
                ),
                const SizedBox(width: 16),

                // Right: Detailed dossier
                Expanded(
                  flex: 3,
                  child: Container(
                    padding: const EdgeInsets.all(16),
                    decoration: BoxDecoration(
                      border: Border.all(color: const Color(0x3300E5FF)),
                      color: const Color(0x2200E5FF),
                    ),
                    child: Builder(
                      builder: (context) {
                        if (widget.codexEntries.isEmpty) {
                          return const Center(child: Text('NO ARCHIVAL RECORDS FOUND'));
                        }
                        final entry = widget.codexEntries[_selectedEntryIndex.clamp(0, widget.codexEntries.length - 1)];
                        return SingleChildScrollView(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                entry.title,
                                style: const TextStyle(color: Color(0xFF00E5FF), fontSize: 18, fontWeight: FontWeight.bold),
                              ),
                              const SizedBox(height: 4),
                              Text(
                                'FACTION: ${entry.faction} | CLASSIFICATION: ${entry.classification}',
                                style: const TextStyle(color: Color(0x9900E5FF), fontSize: 11),
                              ),
                              const SizedBox(height: 12),
                              Text(
                                entry.excerpt,
                                style: const TextStyle(color: Color(0xFFFFFFFF), fontSize: 13, height: 1.5),
                              ),
                            ],
                          ),
                        );
                      },
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
