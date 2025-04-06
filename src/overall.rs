use crate::Individual;

pub fn sum_points(individuals: Vec<Individual>) -> f32 {
    let mut total = 0.0;
    for individual in individuals.iter() {
        total += individual.points;
    }

    total
}
