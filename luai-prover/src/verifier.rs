//! `Verifier` — verify zkVM proofs and extract public inputs.

use luai::zkvm::commitment::PublicInputs;

/// Verifies zkVM proofs and extracts `PublicInputs` from the journal.
///
/// NOTE: Actual proof verification requires RISC Zero (`receipt.verify(LUAI_GUEST_ID)`).
/// The methods here are stubs that document the verification API.
pub struct Verifier;

impl Verifier {
    pub fn new() -> Self {
        Verifier
    }

    // ── Verification (requires RISC Zero) ─────────────────────────────────────
    //
    // pub fn verify(&self, receipt: &Receipt) -> Result<PublicInputs, VerificationError> {
    //     receipt.verify(LUAI_GUEST_ID)?;
    //     let public_inputs: PublicInputs = receipt.journal.decode()?;
    //     Ok(public_inputs)
    // }
    //
    // pub fn verify_with_expected(
    //     &self,
    //     receipt: &Receipt,
    //     expected: &PublicInputs,
    // ) -> Result<(), VerificationError> {
    //     let actual = self.verify(receipt)?;
    //     if actual != *expected {
    //         return Err(VerificationError::PublicInputsMismatch { actual, expected: expected.clone() });
    //     }
    //     Ok(())
    // }

    /// Check that a `PublicInputs` matches expected values (for testing without RISC Zero).
    pub fn check_public_inputs(
        actual: &PublicInputs,
        expected: &PublicInputs,
    ) -> Result<(), String> {
        if actual != expected {
            Err(format!(
                "PublicInputs mismatch:\n  actual:   {:?}\n  expected: {:?}",
                actual, expected
            ))
        } else {
            Ok(())
        }
    }
}

impl Default for Verifier {
    fn default() -> Self {
        Verifier::new()
    }
}
