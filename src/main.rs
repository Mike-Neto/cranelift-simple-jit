use jit_test::{build_and_run_jit, build_native};

fn main() -> Result<(), String> {
    let operands: Vec<i64> = std::env::args()
        .skip(1)
        .filter_map(|n| n.parse().ok())
        .collect();

    if operands.len() != 2 {
        return Err(String::from(
            "You need to pass 2 numbers to jit compile into a adder function cargo run -- 1 2",
        ));
    }

    build_and_run_jit(operands.clone())?;

    build_native(operands.clone())?;

    Ok(())
}
