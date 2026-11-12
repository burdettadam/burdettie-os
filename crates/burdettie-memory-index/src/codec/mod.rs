//! Vector codec versioning and baseline compression for P3 tiered storage.
//!
//! Each codec is identified by a version string and can encode/decode sparse
//! term-weight vectors.  The baseline `scalar_v1` codec uses scalar quantization
//! (u8 per weight) plus L2-norm tracking for cosine-aware storage.
//!
//! TurboQuant-style warm-tier codecs will be added here as the next version
//! once the baseline compression path is measured and proven.

use crate::vector::TermWeight;

/// Identifies a specific vector codec and its version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecVersion {
    /// Short identifier, e.g. `"scalar_v1"`, `"turboquant_v1"`.
    pub name: String,
    /// Monotonic version number for migration ordering.
    pub version: u32,
}

impl CodecVersion {
    pub fn scalar_v1() -> Self {
        Self {
            name: "scalar_v1".to_string(),
            version: 1,
        }
    }

    pub fn active() -> Self {
        Self::scalar_v1()
    }

    pub fn as_str(&self) -> String {
        format!("{}:{}", self.name, self.version)
    }
}

/// Compression result with metrics.
#[derive(Debug, Clone)]
pub struct CompressedVector {
    /// The codec that produced this representation.
    pub codec: CodecVersion,
    /// Serialized compressed payload.
    pub data: Vec<u8>,
}

impl CompressedVector {
    /// Compress a sparse term-weight vector using the active codec.
    pub fn encode(entries: &[TermWeight]) -> Self {
        encode_scalar_v1(entries)
    }

    /// Decompress back to sparse term-weight vector.
    pub fn decode(&self) -> Vec<TermWeight> {
        match self.codec.name.as_str() {
            "scalar_v1" => decode_scalar_v1(&self.data),
            _ => Vec::new(),
        }
    }

    /// Bytes on disk for this compressed vector.
    pub fn byte_length(&self) -> usize {
        self.data.len()
    }

    /// Codec version string suitable for storage.
    pub fn codec_version_str(&self) -> String {
        self.codec.as_str()
    }
}

/// Compression/decompression statistics for a corpus.
#[derive(Debug, Clone, Default)]
pub struct CompressionStats {
    pub codec_version: String,
    pub total_vectors: usize,
    pub total_raw_bytes: u64,
    pub total_compressed_bytes: u64,
    /// Compression ratio: raw / compressed (higher is better).
    pub ratio: f64,
    /// Bytes per vector (compressed).
    pub bytes_per_vector: f64,
}

impl CompressionStats {
    pub fn compute(
        codec_version: String,
        raw_vectors: &[Vec<TermWeight>],
        compressed: &[CompressedVector],
    ) -> Self {
        let total_raw_bytes: u64 = raw_vectors.iter().map(|v| (v.len() * 8) as u64).sum();
        let total_compressed_bytes: u64 = compressed.iter().map(|c| c.data.len() as u64).sum();
        let ratio = if total_compressed_bytes > 0 {
            total_raw_bytes as f64 / total_compressed_bytes as f64
        } else {
            0.0
        };
        let bytes_per_vector = if compressed.is_empty() {
            0.0
        } else {
            total_compressed_bytes as f64 / compressed.len() as f64
        };

        Self {
            codec_version,
            total_vectors: compressed.len(),
            total_raw_bytes,
            total_compressed_bytes,
            ratio,
            bytes_per_vector,
        }
    }
}

// ── scalar_v1 codec ─────────────────────────────────────────────

/// Encode sparse term-weight pairs into scalar-quantized bytes.
///
/// Layout per entry:
///   [term_index: 4 bytes LE][quant_weight: 1 byte u8]
///
/// Weights are quantized to [0, 255] via `(weight * 127.5 + 127.5).round().clamp(0, 255)`.
/// This maps the [-1, 1] range to u8 with ~0.008 precision.
fn encode_scalar_v1(entries: &[TermWeight]) -> CompressedVector {
    let mut data = Vec::with_capacity(entries.len() * 5);
    for entry in entries {
        data.extend_from_slice(&entry.term_index.to_le_bytes());
        let quant = ((entry.weight * 127.5 + 127.5).round() as u32).clamp(0, 255) as u8;
        data.push(quant);
    }
    CompressedVector {
        codec: CodecVersion::scalar_v1(),
        data,
    }
}

/// Decode scalar_v1 back to sparse term-weight pairs.
fn decode_scalar_v1(data: &[u8]) -> Vec<TermWeight> {
    let mut entries = Vec::with_capacity(data.len() / 5);
    for chunk in data.chunks_exact(5) {
        let term_index = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let quant = chunk[4];
        // Dequantize: [0, 255] → [-1, 1]
        let weight = (quant as f32 - 127.5) / 127.5;
        entries.push(TermWeight { term_index, weight });
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_v1_roundtrip_preserves_order() {
        let original = vec![
            TermWeight {
                term_index: 5,
                weight: 0.866,
            },
            TermWeight {
                term_index: 12,
                weight: -0.5,
            },
            TermWeight {
                term_index: 0,
                weight: 0.0,
            },
        ];
        let compressed = encode_scalar_v1(&original);
        let restored = compressed.decode();

        assert_eq!(restored.len(), original.len());
        for (orig, rest) in original.iter().zip(restored.iter()) {
            assert_eq!(orig.term_index, rest.term_index);
            // Check reconstruction is within ~0.01 (8-bit quantization error)
            assert!(
                (orig.weight - rest.weight).abs() < 0.02,
                "weight {} → {} outside tolerance",
                orig.weight,
                rest.weight
            );
        }
    }

    #[test]
    fn scalar_v1_compresses_below_raw() {
        // 10 entries → raw = 80 bytes, compressed = 50 bytes
        let entries: Vec<TermWeight> = (0..10)
            .map(|i| TermWeight {
                term_index: i,
                weight: 0.5,
            })
            .collect();
        let compressed = encode_scalar_v1(&entries);
        let raw_bytes = (entries.len() * 8) as u64;

        assert!(
            compressed.data.len() < raw_bytes as usize,
            "scalar_v1 should compress: {} < {}",
            compressed.data.len(),
            raw_bytes
        );
    }

    #[test]
    fn compression_stats_computes_ratio() {
        let vectors: Vec<Vec<TermWeight>> = (0..5)
            .map(|_| {
                (0..20)
                    .map(|i| TermWeight {
                        term_index: i,
                        weight: 0.5,
                    })
                    .collect()
            })
            .collect();
        let compressed: Vec<CompressedVector> =
            vectors.iter().map(|v| encode_scalar_v1(v)).collect();
        let stats = CompressionStats::compute("scalar_v1:1".to_string(), &vectors, &compressed);

        assert_eq!(stats.total_vectors, 5);
        assert!(stats.ratio > 1.0, "should compress: ratio={}", stats.ratio);
        assert!(stats.bytes_per_vector > 0.0);
        assert!(stats.bytes_per_vector < 160.0); // raw is 160 per vector
    }
}
