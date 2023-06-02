//! Macros

/// Generates the signer seeds for a GaugeFactory.
#[macro_export]
macro_rules! gauge_factory_seeds {
    ($gf: expr) => {
        &[&[b"GaugeFactory" as &[u8], &$gf.base.to_bytes(), &[$gf.bump]]]
    };
}
