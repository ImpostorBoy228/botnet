use integrate::newton_cotes::trapezoidal_rule;

fn main() {
    let feh = |x: f64| x.powi(2) + 4.0 * x + 5.0;
    let res = trapezoidal_rule(feh, -2.0, 2.0, 100_000_000_u32);
    println!("{res}");
}
