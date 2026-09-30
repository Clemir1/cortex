//! A/A da física paralela (17.6): o rng derivado de (seed, id, step)
//! torna o resultado INDEPENDENTE da ordem de execução — provado
//! bit-exato comparando os bits de estado/energia/posição/tau de runs
//! completas sob POOLS DE THREADS DIFERENTES (determinismo por
//! construção, não por ordenar efeitos).

use rayon::ThreadPoolBuilder;
use triad_l1_substrate::L1Runner;

fn snapshot_bits(r: &L1Runner) -> Vec<Vec<u64>> {
    r.clusters
        .iter()
        .map(|c| {
            let mut v: Vec<u64> = c.state.iter().map(|x| x.to_bits()).collect();
            v.push(c.energy.to_bits());
            v.extend(c.position.iter().map(|x| x.to_bits()));
            v.push(c.tau_age.to_bits());
            v
        })
        .collect()
}

fn run(threads: usize) -> (Vec<Vec<u64>>, (u64, u64, u64)) {
    let pool = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap();
    pool.install(|| {
        let mut r = L1Runner::new(42, 400);
        for _ in 0..12 {
            let _ = r.step();
        }
        (
            snapshot_bits(&r),
            (r.divisions_total, r.deaths_total, r.merges_total),
        )
    })
}

#[test]
fn fisica_paralela_deterministica_entre_threads() {
    let (bits1, c1) = run(1);
    let (bits4, c4) = run(4);
    let (bits8, c8) = run(8);
    assert_eq!(bits1, bits4, "A/A: 1 thread vs 4 threads devem ser bit-idênticos");
    assert_eq!(bits1, bits8, "A/A: 1 thread vs 8 threads devem ser bit-idênticos");
    assert_eq!(c1, c4);
    assert_eq!(c1, c8);
    // run-vs-run (mesma configuração): determinismo da gênese + física.
    let (a, ca) = run(4);
    let (b, cb) = run(4);
    assert_eq!(a, b, "A/A: duas runs com mesma seed devem ser bit-idênticas");
    assert_eq!(ca, cb);
}
