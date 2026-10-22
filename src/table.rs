//! The shape of UCUM's atom and prefix table. The rows themselves are in
//! `ucum_table.generated.rs`, generated from the essence file that ucum-units bundles.

/// A UCUM prefix: its code and its value as UCUM writes it (`1e3`).
#[derive(Debug)]
pub(crate) struct Prefix {
    pub(crate) code: &'static str,
    pub(crate) value: &'static str,
}

/// The name of the function that defines a special unit, such as `Cel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Special(pub(crate) &'static str);

/// A UCUM unit atom other than the seven base units.
///
/// For an ordinary atom, one of it is `value` times `unit`. For a special
/// atom, `value` and `unit` are the function's proper unit, which gives its
/// dimension.
#[derive(Debug)]
pub(crate) struct Atom {
    pub(crate) code: &'static str,
    pub(crate) metric: bool,
    pub(crate) arbitrary: bool,
    pub(crate) special: Option<Special>,
    pub(crate) value: &'static str,
    pub(crate) unit: &'static str,
}
