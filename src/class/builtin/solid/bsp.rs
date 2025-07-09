use super::*;

/// Merged into `worldspawn` on compile. Acts as world geometry, except doesn't split vis leafs.
/// 
/// NOTE: Currently, vis data is not used, so this is used more for grouping.
#[solid_class(base(BspSolidEntity))]
#[derive(Debug, Clone)]
pub struct FuncDetail;
