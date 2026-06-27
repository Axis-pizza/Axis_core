use pinocchio::Address;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteDirection {
    Composition,
    Unwind,
}

// TODO(account-model): Define route granularity and validation before CPI use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovedRoute {
    pub venue_program: Address,
    pub venue_account: Address,
    pub input_mint: Address,
    pub output_mint: Address,
    pub direction: RouteDirection,
    pub enabled: bool,
}
