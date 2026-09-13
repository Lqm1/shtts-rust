# Test fixtures

- `ref/`: expected PCM and case metadata.
- `cases.tsv`: input text and voice settings for `ref/`.
- `python/`: additional expected PCM and its `cases.tsv` manifest.
- `kernel/coefficients.bin`: expected LPC coefficients.
- `kernel/blocks.bin`: expected excitation blocks.
- `kernel/utterances.bin`: analyzed segments for synthesis tests.

PCM is mono signed 16-bit little-endian at 11,025 Hz. Tests require exact
sample and length matches. These files are used only by tests.
