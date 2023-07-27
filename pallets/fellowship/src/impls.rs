use crate::traits::EnsureRole;
use crate::*;
use common_traits::MaybeConvert;
use frame_support::ensure;
use sp_runtime::{
    traits::{BadOrigin, Convert},
    Percent,
};
use sp_std::vec::Vec;

/// Ensure that a account is of a given role.
/// Used in other pallets like an ensure origin.
pub struct EnsureFellowshipRole<T>(T);
impl<T: Config> EnsureRole<AccountIdOf<T>, Role> for EnsureFellowshipRole<T> {
    type Success = ();

    fn ensure_role(acc: &AccountIdOf<T>, role: Role) -> Result<Self::Success, BadOrigin> {
        let actual = Roles::<T>::get(acc).ok_or(BadOrigin)?;
        if role == actual {
            Ok(())
        } else {
            Err(BadOrigin)
        }
    }
    fn ensure_role_in(acc: &AccountIdOf<T>, roles: Vec<Role>) -> Result<Self::Success, BadOrigin> {
        let role = Roles::<T>::get(acc).ok_or(BadOrigin)?;
        ensure!(roles.contains(&role), BadOrigin);
        Ok(())
    }
}

impl<T: Config> MaybeConvert<&AccountIdOf<T>, VetterIdOf<T>> for Pallet<T> {
    fn maybe_convert(fellow: &AccountIdOf<T>) -> Option<VetterIdOf<T>> {
        FellowToVetter::<T>::get(fellow)
    }
}

pub struct RoleToPercentFee;
impl Convert<crate::Role, Percent> for RoleToPercentFee {
    fn convert(role: Role) -> Percent {
        match role {
            Role::Vetter => Percent::from_percent(50u8),
            Role::Freelancer => Percent::from_percent(50u8),
            Role::BusinessDev => Percent::from_percent(50u8),
            Role::Approver => Percent::from_percent(50u8),
        }
    }
}
