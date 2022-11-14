#![feature(generic_const_exprs)]

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkGroup};

use raid::distributed::HeadNode;
use raid::file::FileHandler;
use raid::raid::RAID;
use raid::single::SingleServer;
use std::path::PathBuf;
use rand::{RngCore, Rng};
use criterion::measurement::Measurement;
use std::time::Duration;


fn criterion_benches(c: &mut Criterion) {
    const X: usize = usize::pow(2, 2);
    criterion_read::<6, 2, X, _>(c.benchmark_group("read"));
    criterion_write::<6, 2, X, _>(c.benchmark_group("write"));
    criterion_recover::<6, 2, X, _>(c.benchmark_group("recover"), 2);
}

fn criterion_read<const D: usize, const C: usize, const X: usize, M: Measurement + 'static>(mut group: BenchmarkGroup<M>)
    where
        [(); X * D]:,
        [(); D * X]:,
        [(); C + D]:,
        [(); D + C]:,
        [(); C + C]:,
        [(); D + D]:,
{
    group.sample_size(10).measurement_time(Duration::from_secs(10));
    let file_handler = prepare_read::<SingleServer<D, C, X>, D, C, X>();
    group.bench_function("single read small", |b| b.iter(|| file_handler.read_file("small3")));
    group.bench_function("single read normal", |b| b.iter(|| file_handler.read_file("normal3")));
    group.bench_function("single read large", |b| b.iter(|| file_handler.read_file("large3")));
    file_handler.shutdown();

    let file_handler = prepare_read::<HeadNode<D, C, X>, D, C, X>();
    group.bench_function("distributed read small", |b| b.iter(|| file_handler.read_file("small3")));
    group.bench_function("distributed read normal", |b| b.iter(|| file_handler.read_file("normal3")));
    group.bench_function("distributed read large", |b| b.iter(|| file_handler.read_file("large3")));
    file_handler.shutdown();
    group.finish();
}


fn criterion_write<const D: usize, const C: usize, const X: usize, M: Measurement + 'static>(mut group: BenchmarkGroup<M>)
    where
        [(); X * D]:,
        [(); D * X]:,
        [(); C + D]:,
        [(); D + C]:,
        [(); C + C]:,
        [(); D + D]:,
{
    group.sample_size(10).measurement_time(Duration::from_secs(20));

    let mut small = vec![0u8; X / 2];
    let mut normal = vec![0u8; X * D];
    let mut large = vec![0u8; X * D * 10];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut small);
    rng.fill_bytes(&mut normal);
    rng.fill_bytes(&mut large);

    let mut file_handler = prepare_read::<SingleServer<D, C, X>, D, C, X>();
    group.bench_function("single write small", |b| b.iter(|| file_handler.add_file("s".to_string(), &small)));
    group.bench_function("single write normal", |b| b.iter(|| file_handler.add_file("n".to_string(), &normal)));
    group.bench_function("single write large", |b| b.iter(|| file_handler.add_file("l".to_string(), &large)));
    file_handler.shutdown();

    let mut file_handler = prepare_read::<HeadNode<D, C, X>, D, C, X>();
    group.bench_function("distributed write small", |b| b.iter(|| file_handler.add_file("s".to_string(), &small)));
    group.bench_function("distributed write normal", |b| b.iter(|| file_handler.add_file("n".to_string(), &normal)));
    group.bench_function("distributed write large", |b| b.iter(|| file_handler.add_file("l".to_string(), &large)));
    file_handler.shutdown();
    group.finish();
}


fn criterion_recover<const D: usize, const C: usize, const X: usize, M: Measurement + 'static>(mut group: BenchmarkGroup<M>, failures: usize)
    where
        [(); X * D]:,
        [(); D * X]:,
        [(); C + D]:,
        [(); D + C]:,
        [(); C + C]:,
        [(); D + D]:,
{
    group.sample_size(10).measurement_time(Duration::from_secs(30));
    let failures: Vec<_> = (0..failures).collect();

    let file_handler = prepare_read::<SingleServer<D, C, X>, D, C, X>();
    group.bench_function("single recover", |b| b.iter(|| file_handler.destroy_devices(&failures)));
    file_handler.shutdown();
    let file_handler = prepare_read::<HeadNode<D, C, X>, D, C, X>();
    group.bench_function("distributed recover", |b| b.iter(|| {
        file_handler.destroy_devices(&failures);
        file_handler.read_file("large1")
    }));
    file_handler.shutdown();
    group.finish()
}

fn prepare_read<R, const D: usize, const C: usize, const X: usize>() -> FileHandler<R, D, C, X> where
    R: RAID<D, C, X>,
    [(); X * D]:,
    [(); D * X]:,
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    let mut rng = rand::thread_rng();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("nodes");
    let mut file_handler: FileHandler<R, D, C, X> =
        FileHandler::new(path);

    let lengths = vec![("small", X / 2), ("normal", X * D), ("large", X * D * 10)];

    for i in 0..5 {
        for (name, length) in &lengths {
            let mut content = vec![0u8; *length];
            rng.fill_bytes(&mut content);
            file_handler.add_file(format!("{name}{i}"), &content);
        }
    }

    file_handler
}


criterion_group!(benches, criterion_benches);
criterion_main!(benches);
