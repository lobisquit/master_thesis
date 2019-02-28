
pub fn utility(value: f64,
               critic_value: f64,
               tolerance: f64,
               margin: f64) -> f64 {

    // Utility function is a sigmoid guaranteed to cross (critic_value, 0.5) and
    // (critic_value + tolerance, margin)
    1./(1. + ((1. - margin)/margin).powf((value - critic_value)/tolerance))
}
