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
    criterion_write(c.benchmark_group("write"));
    criterion_read(c.benchmark_group("read"));
    criterion_recover(c.benchmark_group("recover"));
}

fn criterion_read<M: Measurement + 'static>(mut group: BenchmarkGroup<M>) {

    group.sample_size(100).measurement_time(Duration::from_secs(10));
    let file_handler = prepare_read::<SingleServer<6, 2, 4194304>, 6, 2, 4194304>();
    group.bench_function("single read small", |b| b.iter(|| file_handler.read_file("small3")));
    group.bench_function("single read normal", |b| b.iter(|| file_handler.read_file("normal3")));
    group.bench_function("single read large", |b| b.iter(|| file_handler.read_file("large3")));
    file_handler.shutdown();

    let file_handler = prepare_read::<HeadNode<6, 2, 4194304>, 6, 2, 4194304>();
    group.bench_function("distributed read small", |b| b.iter(|| file_handler.read_file("small3")));
    group.bench_function("distributed read normal", |b| b.iter(|| file_handler.read_file("normal3")));
    group.bench_function("distributed read large", |b| b.iter(|| file_handler.read_file("large3")));
    file_handler.shutdown();
    group.finish();
}


fn criterion_write<M: Measurement + 'static>(mut group: BenchmarkGroup<M>) {
    const X: usize = 4194304; // 4MB
    const D: usize = 6;
    const C: usize = 2;

    group.sample_size(100).measurement_time(Duration::from_secs(20));

    let mut small = vec![0u8; X / 2];
    let mut normal = vec![0u8; X * (D + C)];
    let mut large = vec![0u8; X * (D + C) * 10];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut small);
    rng.fill_bytes(&mut normal);
    rng.fill_bytes(&mut large);

    let mut file_handler = prepare_read::<SingleServer<6, 2, 4194304>, 6, 2, 4194304>();
    group.bench_function("single write small", |b| b.iter(|| file_handler.add_file("s".to_string(), &small)));
    group.bench_function("single write normal", |b| b.iter(|| file_handler.add_file("n".to_string(), &normal)));
    group.bench_function("single write large", |b| b.iter(|| file_handler.add_file("l".to_string(), &large)));
    file_handler.shutdown();

    let mut file_handler = prepare_read::<HeadNode<6, 2, 4194304>, 6, 2, 4194304>();
    group.bench_function("distributed write small", |b| b.iter(|| file_handler.add_file("s".to_string(), &small)));
    group.bench_function("distributed write normal", |b| b.iter(|| file_handler.add_file("n".to_string(), &normal)));
    group.bench_function("distributed write large", |b| b.iter(|| file_handler.add_file("l".to_string(), &large)));
    file_handler.shutdown();
    group.finish();
}


fn criterion_recover<M: Measurement + 'static>(mut group: BenchmarkGroup<M>) {
    const X: usize = 4194304; // 4MB
    const D: usize = 6;
    const C: usize = 2;

    group.sample_size(10).measurement_time(Duration::from_secs(30));;

    let file_handler = prepare_read::<SingleServer<6, 2, 4194304>, 6, 2, 4194304>();
    group.bench_function("single recover", |b| b.iter(|| file_handler.destroy_devices(&vec![0, 1])));
    file_handler.shutdown();
    let file_handler = prepare_read::<HeadNode<6, 2, 4194304>, 6, 2, 4194304>();
    group.bench_function("distributed recover", |b| b.iter(|| {
        file_handler.destroy_devices(&vec![0, 1]);
        file_handler.read_file("large1")
    }));
    file_handler.shutdown();
    group.finish()
}

fn prepare_read<R, const D: usize, const C: usize, const X: usize>() -> FileHandler<R, D, C, X> where
    R: RAID<D, C, X>,
    [(); X * D]:,
    [(); D * X]:,
    [(); X * D]:,
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    let mut rng = rand::thread_rng();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("nodes");
    let mut file_handler: FileHandler<R, D, C, X> =
        FileHandler::new(path);

    let lengths = vec![("small", X / 2), ("normal", X * (D + C)), ("large", X * (D + C) * 10)];

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
