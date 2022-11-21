#![feature(generic_const_exprs)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkGroup, Criterion};

use criterion::measurement::Measurement;
use raid::file::FileHandler;
use raid::raid::distributed::HeadNode;
use raid::raid::single::SingleServer;
use raid::raid::RAID;
use rand::seq::SliceRandom;
use rand::{Rng, RngCore, SeedableRng};
use seq_macro::seq;
use std::path::PathBuf;
use std::time::Duration;

const SAMPLE_POINTS: usize = 10;

fn criterion_benches(c: &mut Criterion) {
    const X: usize = usize::pow(2, 20);
    
    criterion_read::<6, 2, X, _>(c.benchmark_group("read"));
    criterion_write::<6, 2, X, _>(c.benchmark_group("write"));

    criterion_recover::<6, 1, X, _>(c.benchmark_group("recover611"), 1);

    criterion_recover::<6, 2, X, _>(c.benchmark_group("recover621"), 1);
    criterion_recover::<6, 2, X, _>(c.benchmark_group("recover622"), 2);

    criterion_recover::<6, 3, X, _>(c.benchmark_group("recover631"), 1);
    criterion_recover::<6, 3, X, _>(c.benchmark_group("recover632"), 2);
    criterion_recover::<6, 3, X, _>(c.benchmark_group("recover633"), 3);

    criterion_recover::<6, 4, X, _>(c.benchmark_group("recover641"), 1);
    criterion_recover::<6, 4, X, _>(c.benchmark_group("recover642"), 2);
    criterion_recover::<6, 4, X, _>(c.benchmark_group("recover643"), 3);
    criterion_recover::<6, 4, X, _>(c.benchmark_group("recover644"), 4);

    criterion_recover::<6, 5, X, _>(c.benchmark_group("recover651"), 1);
    criterion_recover::<6, 5, X, _>(c.benchmark_group("recover652"), 2);
    criterion_recover::<6, 5, X, _>(c.benchmark_group("recover653"), 3);
    criterion_recover::<6, 5, X, _>(c.benchmark_group("recover654"), 4);
    criterion_recover::<6, 5, X, _>(c.benchmark_group("recover655"), 5);

    criterion_recover::<6, 6, X, _>(c.benchmark_group("recover661"), 1);
    criterion_recover::<6, 6, X, _>(c.benchmark_group("recover662"), 2);
    criterion_recover::<6, 6, X, _>(c.benchmark_group("recover663"), 3);
    criterion_recover::<6, 6, X, _>(c.benchmark_group("recover664"), 4);
    criterion_recover::<6, 6, X, _>(c.benchmark_group("recover665"), 5);
    criterion_recover::<6, 6, X, _>(c.benchmark_group("recover666"), 6);

    criterion_write_single::<6, 0, X, _>(c.benchmark_group("cwrite60"));
    criterion_write_single::<6, 1, X, _>(c.benchmark_group("cwrite61"));
    criterion_write_single::<6, 2, X, _>(c.benchmark_group("cwrite62"));
    criterion_write_single::<6, 3, X, _>(c.benchmark_group("cwrite63"));
    criterion_write_single::<6, 4, X, _>(c.benchmark_group("cwrite64"));
    criterion_write_single::<6, 5, X, _>(c.benchmark_group("cwrite65"));
    criterion_write_single::<6, 6, X, _>(c.benchmark_group("cwrite66"));
    
    
    seq!(N in 2..=100 {
        criterion_write_single::<N, 2, X, _>(c.benchmark_group(format!("dwrite_{}_2", N)));
    });
    
    criterion_read_single::<6, 0, X, _>(c.benchmark_group("cread60"));
    criterion_read_single::<6, 1, X, _>(c.benchmark_group("cread61"));
    criterion_read_single::<6, 2, X, _>(c.benchmark_group("cread62"));
    criterion_read_single::<6, 3, X, _>(c.benchmark_group("cread63"));
    criterion_read_single::<6, 4, X, _>(c.benchmark_group("cread64"));
    criterion_read_single::<6, 5, X, _>(c.benchmark_group("cread65"));
    criterion_read_single::<6, 6, X, _>(c.benchmark_group("cread66"));
    
    seq!(N in 2..=100 {
        criterion_read_single::<N, 2, X, _>(c.benchmark_group(format!("dread_{}_2", N)));
    });
    
    seq!(N in 2..=60 {
        criterion_recover::<N, 2, X, _>(c.benchmark_group(format!("drecover_{}_2_1", N)), 1);
        criterion_recover::<N, 2, X, _>(c.benchmark_group(format!("drecover_{}_2_2", N)), 2);
    });
    
}

fn criterion_read<const D: usize, const C: usize, const X: usize, M: Measurement + 'static>(
    mut group: BenchmarkGroup<M>,
) where
    [(); X * D]:,
    [(); D * X]:,
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    let lengths: [usize; SAMPLE_POINTS] = core::array::from_fn(|i| {
        ((100 * 6 - 1) * X * i + X * SAMPLE_POINTS) / (10 * SAMPLE_POINTS)
    });

    group
        .sample_size(100)
        .measurement_time(Duration::from_nanos(1));
    let file_handler = prepare_read::<SingleServer<D, C, X>, D, C, X>();
    for length in &lengths {
        group.bench_function(format!("single_{length}"), |b| {
            b.iter(|| file_handler.read_file(&format!("{length}")))
        });
    }
    file_handler.shutdown();

    let file_handler = prepare_read::<HeadNode<D, C, X>, D, C, X>();
    for length in &lengths {
        group.bench_function(format!("dist_{length}"), |b| {
            b.iter(|| file_handler.read_file(&format!("{length}")))
        });
    }
    file_handler.shutdown();
    group.finish();
}
fn criterion_read_single<const D: usize, const C: usize, const X: usize, M: Measurement + 'static>(
    mut group: BenchmarkGroup<M>,
) where
    [(); X * D]:,
    [(); D * X]:,
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{

    group
        .sample_size(100)
        .measurement_time(Duration::from_nanos(1));
    let file_handler = prepare_read::<SingleServer<D, C, X>, D, C, X>();
    
    let length = ((100 * 6 - 1) * X / 2 + X) / (10);
    group.bench_function(format!("single_{length}"), |b| {
        b.iter(|| file_handler.read_file(&format!("{length}")))
    });
    file_handler.shutdown();

    let file_handler = prepare_read::<HeadNode<D, C, X>, D, C, X>();
    group.bench_function(format!("dist_{length}"), |b| {
        b.iter(|| file_handler.read_file(&format!("{length}")))
    });
    file_handler.shutdown();
    group.finish();
}

fn criterion_write<const D: usize, const C: usize, const X: usize, M: Measurement + 'static>(
    mut group: BenchmarkGroup<M>,
) where
    [(); X * D]:,
    [(); D * X]:,
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    group
        .sample_size(100)
        .measurement_time(Duration::from_nanos(1));

    let mut rng = rand::rngs::StdRng::seed_from_u64(2);

    let mut files: [Vec<u8>; SAMPLE_POINTS] = core::array::from_fn(|i| {
        let length = ((100 * 6 - 1) * X * i + X * SAMPLE_POINTS) / (10 * SAMPLE_POINTS);
        let mut vec = vec![0u8; length];
        rng.fill_bytes(&mut vec);
        vec
    });

    files.shuffle(&mut rng);

    let mut file_handler = prepare_read::<SingleServer<D, C, X>, D, C, X>();
    for file in &files {
        group.bench_function(format!("single_{}", file.len()), |b| {
            b.iter(|| {
                file_handler.add_file("s".to_string(), file);
                file_handler.ping();
            })
        });
    }
    file_handler.shutdown();

    let mut file_handler = prepare_read::<HeadNode<D, C, X>, D, C, X>();
    for file in &files {
        group.bench_function(format!("dist_{}", file.len()), |b| {
            b.iter(|| {
                file_handler.add_file("s".to_string(), file);
                file_handler.ping();
            })
        });
    }
    file_handler.shutdown();
    group.finish();
}

fn criterion_write_single<
    const D: usize,
    const C: usize,
    const X: usize,
    M: Measurement + 'static,
>(
    mut group: BenchmarkGroup<M>,
) where
    [(); X * D]:,
    [(); D * X]:,
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    group
        .sample_size(100)
        .measurement_time(Duration::from_nanos(1));

    let mut rng = rand::rngs::StdRng::seed_from_u64(2);

    let length = ((100 * 6 - 1) * X / 2 + X) / (10);
    let mut file = vec![0u8; length];
    rng.fill_bytes(&mut file);

    let mut file_handler = prepare_read::<SingleServer<D, C, X>, D, C, X>();
    group.bench_function(format!("single_{}", file.len()), |b| {
        b.iter(|| {
            file_handler.add_file("s".to_string(), &file);
            file_handler.ping();
        })
    });
    file_handler.shutdown();

    let mut file_handler = prepare_read::<HeadNode<D, C, X>, D, C, X>();
    group.bench_function(format!("dist_{}", file.len()), |b| {
        b.iter(|| {
            file_handler.add_file("s".to_string(), &file);
            file_handler.ping();
        })
    });
    file_handler.shutdown();
    group.finish();
}

fn criterion_recover<const D: usize, const C: usize, const X: usize, M: Measurement + 'static>(
    mut group: BenchmarkGroup<M>,
    failures: usize,
) where
    [(); X * D]:,
    [(); D * X]:,
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    group
        .sample_size(20)
        .measurement_time(Duration::from_nanos(1));
    let failures: Vec<_> = (0..failures).collect();

    let file_handler = prepare_read::<SingleServer<D, C, X>, D, C, X>();
    group.bench_function("single recover", |b| {
        b.iter(|| {
            file_handler.destroy_devices(&failures);
            file_handler.ping();
        })
    });
    file_handler.shutdown();
    let file_handler = prepare_read::<HeadNode<D, C, X>, D, C, X>();
    group.bench_function("distributed recover", |b| {
        b.iter(|| {
            file_handler.destroy_devices(&failures);
            file_handler.ping();
        })
    });
    file_handler.shutdown();
    group.finish()
}

fn prepare_read<R, const D: usize, const C: usize, const X: usize>() -> FileHandler<R, D, C, X>
where
    R: RAID<D, C, X>,
    [(); X * D]:,
    [(); D * X]:,
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    let mut rng = rand::rngs::StdRng::seed_from_u64(1);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("nodes");
    let mut file_handler: FileHandler<R, D, C, X> = FileHandler::new(path);
    let mut lengths: [usize; SAMPLE_POINTS] = core::array::from_fn(|i| {
        ((100 * 6 - 1) * X * i + X * SAMPLE_POINTS) / (10 * SAMPLE_POINTS)
    });

    lengths.shuffle(&mut rng);
    let mut total_bytes = 0;
    for length in &lengths {
        total_bytes += length;
        let mut content = vec![0u8; *length];
        rng.fill_bytes(&mut content);
        file_handler.add_file(format!("{length}"), &content);
    }

    println!("number_of_data_chunks_used: {}, total_bytes: {}", file_handler.number_of_data_chunks_used(), total_bytes);

    file_handler
}


criterion_group!(benches, criterion_benches);
criterion_main!(benches);
