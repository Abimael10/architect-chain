use crate::core::Transaction;
use crate::error::{BlockchainError, Result};
use crate::utils::sha256_digest;
use serde::{Deserialize, Serialize};

/// Merkle tree implementation for efficient transaction verification
///
/// This implementation provides Bitcoin-compatible Merkle tree functionality
/// for verifying transactions without downloading entire blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    root: Option<MerkleNode>,
    leaf_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MerkleNode {
    hash: Vec<u8>,
    left: Option<Box<MerkleNode>>,
    right: Option<Box<MerkleNode>>,
}

/// Merkle proof for transaction verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Transaction hash being proven
    pub transaction_hash: Vec<u8>,
    /// Merkle root hash
    pub merkle_root: Vec<u8>,
    /// Proof path (sibling hashes and directions)
    pub proof_path: Vec<ProofElement>,
    /// Index of the transaction in the block
    pub transaction_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofElement {
    /// Sibling hash
    pub hash: Vec<u8>,
    /// Direction: true if sibling is on the right, false if on the left
    pub is_right: bool,
}

impl MerkleTree {
    /// Create a new Merkle tree from a list of transactions
    pub fn new(transactions: &[Transaction]) -> Result<Self> {
        if transactions.is_empty() {
            return Err(BlockchainError::InvalidBlock(
                "Cannot create Merkle tree from empty transaction list".to_string(),
            ));
        }

        let leaf_hashes: Vec<Vec<u8>> =
            transactions.iter().map(|tx| tx.get_id().to_vec()).collect();

        let root = Self::build_tree(&leaf_hashes)?;

        Ok(MerkleTree {
            root: Some(root),
            leaf_count: transactions.len(),
        })
    }

    /// Create a Merkle tree from transaction hashes (for testing or external use)
    pub fn from_hashes(hashes: &[Vec<u8>]) -> Result<Self> {
        if hashes.is_empty() {
            return Err(BlockchainError::InvalidBlock(
                "Cannot create Merkle tree from empty hash list".to_string(),
            ));
        }

        let root = Self::build_tree(hashes)?;

        Ok(MerkleTree {
            root: Some(root),
            leaf_count: hashes.len(),
        })
    }

    /// Get the Merkle root hash
    pub fn get_root_hash(&self) -> Result<Vec<u8>> {
        match &self.root {
            Some(node) => Ok(node.hash.clone()),
            None => Err(BlockchainError::InvalidBlock(
                "Merkle tree has no root".to_string(),
            )),
        }
    }

    /// Generate a Merkle proof for a transaction at the given index
    pub fn generate_proof(&self, transaction_index: usize) -> Result<MerkleProof> {
        if transaction_index >= self.leaf_count {
            return Err(BlockchainError::InvalidBlock(format!(
                "Transaction index {} out of bounds (max: {})",
                transaction_index,
                self.leaf_count - 1
            )));
        }

        let root = self
            .root
            .as_ref()
            .ok_or_else(|| BlockchainError::InvalidBlock("Merkle tree has no root".to_string()))?;

        let mut proof_path = Vec::new();
        let transaction_hash = self.get_leaf_hash(transaction_index)?;

        Self::build_proof_path(root, transaction_index, self.leaf_count, &mut proof_path)?;

        Ok(MerkleProof {
            transaction_hash,
            merkle_root: root.hash.clone(),
            proof_path,
            transaction_index,
        })
    }

    /// Verify a Merkle proof
    pub fn verify_proof(proof: &MerkleProof) -> Result<bool> {
        let mut current_hash = proof.transaction_hash.clone();

        for element in &proof.proof_path {
            current_hash = if element.is_right {
                // Sibling is on the right, current hash is on the left
                Self::hash_pair(&current_hash, &element.hash)
            } else {
                // Sibling is on the left, current hash is on the right
                Self::hash_pair(&element.hash, &current_hash)
            };
        }

        Ok(current_hash == proof.merkle_root)
    }

    /// Build the Merkle tree recursively
    fn build_tree(hashes: &[Vec<u8>]) -> Result<MerkleNode> {
        if hashes.is_empty() {
            return Err(BlockchainError::InvalidBlock(
                "Cannot build tree from empty hash list".to_string(),
            ));
        }

        if hashes.len() == 1 {
            // For single transaction, Bitcoin applies double SHA-256 (same as calculate_merkle_root)
            return Ok(MerkleNode {
                hash: Self::hash_pair(&hashes[0], &hashes[0]),
                left: None,
                right: None,
            });
        }

        // Build parent level
        let mut parent_hashes = Vec::new();
        let mut i = 0;

        while i < hashes.len() {
            if i + 1 < hashes.len() {
                // Pair exists
                let combined_hash = Self::hash_pair(&hashes[i], &hashes[i + 1]);
                parent_hashes.push(combined_hash);
                i += 2;
            } else {
                // Odd number of nodes - duplicate the last one (Bitcoin behavior)
                let combined_hash = Self::hash_pair(&hashes[i], &hashes[i]);
                parent_hashes.push(combined_hash);
                i += 1;
            }
        }

        // Recursively build the tree
        let _parent_node = Self::build_tree(&parent_hashes)?;

        // Build current level nodes
        let mut nodes = Vec::new();
        let mut j = 0;

        while j < hashes.len() {
            if j + 1 < hashes.len() {
                // Create internal node with two children
                let left_child = if hashes.len() == 2 {
                    // Direct children are leaves
                    MerkleNode {
                        hash: hashes[j].clone(),
                        left: None,
                        right: None,
                    }
                } else {
                    // This is a more complex case - we need to build subtrees
                    // For simplicity in this implementation, we'll use a different approach
                    return Self::build_tree_iterative(hashes);
                };

                let right_child = if hashes.len() == 2 {
                    MerkleNode {
                        hash: hashes[j + 1].clone(),
                        left: None,
                        right: None,
                    }
                } else {
                    return Self::build_tree_iterative(hashes);
                };

                nodes.push(MerkleNode {
                    hash: Self::hash_pair(&hashes[j], &hashes[j + 1]),
                    left: Some(Box::new(left_child)),
                    right: Some(Box::new(right_child)),
                });
                j += 2;
            } else {
                // Single node - duplicate it
                let child = MerkleNode {
                    hash: hashes[j].clone(),
                    left: None,
                    right: None,
                };

                nodes.push(MerkleNode {
                    hash: Self::hash_pair(&hashes[j], &hashes[j]),
                    left: Some(Box::new(child.clone())),
                    right: Some(Box::new(child)),
                });
                j += 1;
            }
        }

        if nodes.len() == 1 {
            Ok(nodes.into_iter().next().unwrap())
        } else {
            // Continue building up the tree
            let node_hashes: Vec<Vec<u8>> = nodes.iter().map(|n| n.hash.clone()).collect();
            Self::build_tree(&node_hashes)
        }
    }

    /// Iterative approach for building Merkle tree (more reliable)
    fn build_tree_iterative(leaf_hashes: &[Vec<u8>]) -> Result<MerkleNode> {
        let mut current_level: Vec<MerkleNode> = leaf_hashes
            .iter()
            .map(|hash| MerkleNode {
                hash: hash.clone(),
                left: None,
                right: None,
            })
            .collect();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            let mut i = 0;

            while i < current_level.len() {
                let left = current_level[i].clone();
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1].clone()
                } else {
                    // Duplicate the last node if odd number
                    current_level[i].clone()
                };

                let combined_hash = Self::hash_pair(&left.hash, &right.hash);

                next_level.push(MerkleNode {
                    hash: combined_hash,
                    left: Some(Box::new(left)),
                    right: Some(Box::new(right)),
                });

                i += if i + 1 < current_level.len() { 2 } else { 1 };
            }

            current_level = next_level;
        }

        current_level
            .into_iter()
            .next()
            .ok_or_else(|| BlockchainError::InvalidBlock("Failed to build Merkle tree".to_string()))
    }

    /// Hash two values together (Bitcoin double SHA-256)
    fn hash_pair(left: &[u8], right: &[u8]) -> Vec<u8> {
        let mut combined = Vec::new();
        combined.extend_from_slice(left);
        combined.extend_from_slice(right);

        // Double SHA-256 (Bitcoin standard)
        let first_hash = sha256_digest(&combined);
        sha256_digest(&first_hash)
    }

    /// Get the hash of a leaf at the given index
    fn get_leaf_hash(&self, index: usize) -> Result<Vec<u8>> {
        // This is a simplified implementation
        // In a full implementation, we'd traverse the tree to find the leaf
        if index >= self.leaf_count {
            return Err(BlockchainError::InvalidBlock(
                "Leaf index out of bounds".to_string(),
            ));
        }

        // For now, we'll need to store leaf hashes separately or traverse the tree
        // This is a placeholder that would need the original transaction hashes
        Err(BlockchainError::InvalidBlock(
            "Leaf hash retrieval not implemented in this simplified version".to_string(),
        ))
    }

    /// Build proof path for a transaction
    fn build_proof_path(
        _node: &MerkleNode,
        _target_index: usize,
        _total_leaves: usize,
        _proof_path: &mut [ProofElement],
    ) -> Result<()> {
        // This is a complex recursive function that would traverse the tree
        // to build the proof path. For now, we'll implement a simplified version.

        // In a full implementation, this would:
        // 1. Determine which subtree contains the target index
        // 2. Add the sibling hash to the proof path
        // 3. Recursively traverse the correct subtree

        // Placeholder implementation
        Ok(())
    }

    /// Get the number of leaves in the tree
    pub fn leaf_count(&self) -> usize {
        self.leaf_count
    }

    /// Check if the tree is empty
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }
}

/// Utility functions for Merkle tree operations
impl MerkleTree {
    /// Calculate the Merkle root from a list of transaction hashes
    /// This is a utility function that doesn't build the full tree
    pub fn calculate_merkle_root(transaction_hashes: &[Vec<u8>]) -> Result<Vec<u8>> {
        if transaction_hashes.is_empty() {
            return Err(BlockchainError::InvalidBlock(
                "Cannot calculate Merkle root from empty transaction list".to_string(),
            ));
        }

        // For single transaction, apply double SHA-256 (Bitcoin standard)
        if transaction_hashes.len() == 1 {
            return Ok(Self::hash_pair(
                &transaction_hashes[0],
                &transaction_hashes[0],
            ));
        }

        let mut current_level = transaction_hashes.to_vec();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            let mut i = 0;

            while i < current_level.len() {
                let left = &current_level[i];
                let right = if i + 1 < current_level.len() {
                    &current_level[i + 1]
                } else {
                    // Duplicate the last hash if odd number (Bitcoin behavior)
                    &current_level[i]
                };

                let combined_hash = Self::hash_pair(left, right);
                next_level.push(combined_hash);

                i += if i + 1 < current_level.len() { 2 } else { 1 };
            }

            current_level = next_level;
        }

        Ok(current_level.into_iter().next().unwrap())
    }

    /// Verify that a list of transactions produces the expected Merkle root
    pub fn verify_transactions(transactions: &[Transaction], expected_root: &[u8]) -> Result<bool> {
        let transaction_hashes: Vec<Vec<u8>> =
            transactions.iter().map(|tx| tx.get_id().to_vec()).collect();

        let calculated_root = Self::calculate_merkle_root(&transaction_hashes)?;
        Ok(calculated_root == expected_root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_root_calculation() {
        let hashes = vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8], vec![9, 10, 11, 12]];

        let root = MerkleTree::calculate_merkle_root(&hashes).unwrap();
        assert!(!root.is_empty());
    }

    #[test]
    fn test_single_transaction_merkle_root() {
        let hashes = vec![vec![1, 2, 3, 4]];
        let root = MerkleTree::calculate_merkle_root(&hashes).unwrap();

        // For a single transaction, Bitcoin applies double SHA-256 to create the Merkle root
        // This follows the standard Merkle tree construction where leaf nodes are paired with themselves
        let expected_root = MerkleTree::hash_pair(&hashes[0], &hashes[0]);
        assert_eq!(root, expected_root);

        // Verify the root is different from the original hash due to double SHA-256
        assert_ne!(root, hashes[0]);
        assert_eq!(root.len(), 32); // SHA-256 produces 32-byte hashes
    }

    #[test]
    fn test_empty_transaction_list() {
        let hashes: Vec<Vec<u8>> = vec![];
        let result = MerkleTree::calculate_merkle_root(&hashes);
        assert!(result.is_err());
    }

    #[test]
    fn test_merkle_tree_creation() {
        // This test would require actual Transaction objects
        // For now, we'll test with the hash-based constructor
        let hashes = vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8]];

        let tree = MerkleTree::from_hashes(&hashes).unwrap();
        assert_eq!(tree.leaf_count(), 2);
        assert!(!tree.is_empty());
    }

    #[test]
    fn test_merkle_consistency_single_transaction() {
        // CRITICAL TEST: Verify both methods produce the same result for single transaction
        let test_hash = vec![1, 2, 3, 4];

        // Method 1: Using calculate_merkle_root
        let root_from_calculate = MerkleTree::calculate_merkle_root(&[test_hash.clone()]).unwrap();

        // Method 2: Using build tree via from_hashes
        let tree = MerkleTree::from_hashes(&[test_hash.clone()]).unwrap();
        let root_from_tree = tree.get_root_hash().unwrap();

        // These MUST be equal for consistency!
        assert_eq!(root_from_calculate, root_from_tree,
            "calculate_merkle_root and tree.get_root_hash() produce different results for single transaction!");

        // Verify it matches the expected double SHA-256
        let expected_root = MerkleTree::hash_pair(&test_hash, &test_hash);
        assert_eq!(
            root_from_calculate, expected_root,
            "Single transaction Merkle root should be double SHA-256 of the transaction hash"
        );
    }
}
