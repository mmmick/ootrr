use crate::start_defaults::Edit;

pub const NAME: &str = "Get_StoneOfFire_IsAfter_Ready_Exp";
pub const ANY_FLAG: &str = "Market_Child_TimeTempleDoor_Opened";
const OR_OP: i32 = 4;
const NOT_OP: i32 = 5;

pub fn exp() -> Edit {
    Edit::Exp(NAME.to_string(), vec![(0, ANY_FLAG.to_string()), (0, ANY_FLAG.to_string()), (NOT_OP, String::new()), (OR_OP, String::new())])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_formula_is_a_flag_or_its_negation() {
        let Edit::Exp(n, t) = exp() else { panic!() };
        assert_eq!(n, NAME);
        assert_eq!(t, vec![(0, ANY_FLAG.to_string()), (0, ANY_FLAG.to_string()), (NOT_OP, String::new()), (OR_OP, String::new())]);
    }
}
