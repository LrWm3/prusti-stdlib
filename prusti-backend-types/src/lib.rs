//! Backend-neutral, serializable Prusti verification results.
pub mod java_exception;
pub mod silicon_counterexample;
pub mod verification_backend;
pub mod verification_result;
pub use java_exception::*;
pub use silicon_counterexample::*;
pub use verification_backend::*;
pub use verification_result::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backend_and_result_bincode_discriminants_are_stable() {
        assert_eq!(
            bincode::serialize(&VerificationBackend::Silicon).unwrap(),
            vec![0, 0, 0, 0]
        );
        assert_eq!(
            bincode::serialize(&VerificationBackend::Carbon).unwrap(),
            vec![1, 0, 0, 0]
        );
        assert_eq!(
            bincode::serialize(&VerificationResultKind::Success).unwrap(),
            vec![0, 0, 0, 0]
        );
        assert_eq!(
            bincode::serialize(&VerificationResultKind::Timeout(7)).unwrap(),
            vec![4, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0]
        );
    }
}
