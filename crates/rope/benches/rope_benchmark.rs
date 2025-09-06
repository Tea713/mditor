use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rope::Rope;
use std::hint::black_box;

fn bench_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("creation");

    for size in [100, 1_000, 10_000, 100_000].iter() {
        let text = "a".repeat(*size);

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("rope", size), size, |b, _| {
            b.iter(|| {
                let rope = Rope::from(black_box(text.as_str()));
                black_box(rope);
            })
        });

        group.bench_with_input(BenchmarkId::new("ropey", size), size, |b, _| {
            b.iter(|| {
                let ropey = ropey::Rope::from_str(black_box(text.as_str()));
                black_box(ropey)
            });
        });

        group.bench_with_input(BenchmarkId::new("string", size), size, |b, _| {
            b.iter(|| {
                let string = black_box(text.clone());
                black_box(string);
            })
        });
    }
    group.finish();
}

fn bench_insert_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");

    for size in [1_000, 10_000, 100_000].iter() {
        let text = "a".repeat(*size);
        let insert_text = "INSERTED";

        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::new("rope_beginning", size), size, |b, _| {
            b.iter_batched(
                || Rope::from(text.as_str()),
                |mut rope| {
                    rope.insert(black_box(0), black_box(insert_text));
                    black_box(rope);
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("ropey_beginning", size), size, |b, _| {
            b.iter_batched(
                || ropey::Rope::from(text.as_str()),
                |mut ropey| {
                    ropey.insert(black_box(0), black_box(insert_text));
                    black_box(ropey);
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("string_beginning", size), size, |b, _| {
            b.iter_batched(
                || text.clone(),
                |mut string| {
                    string.insert_str(black_box(0), black_box(insert_text));
                    black_box(string);
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("rope_middle", size), size, |b, _| {
            b.iter_batched(
                || Rope::from(text.as_str()),
                |mut rope| {
                    rope.insert(black_box(size / 2), black_box(insert_text));
                    black_box(rope);
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("ropey_middle", size), size, |b, _| {
            b.iter_batched(
                || ropey::Rope::from(text.as_str()),
                |mut ropey| {
                    ropey.insert(black_box(size / 2), black_box(insert_text));
                    black_box(ropey);
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("string_middle", size), size, |b, _| {
            b.iter_batched(
                || text.clone(),
                |mut string| {
                    string.insert_str(black_box(size / 2), black_box(insert_text));
                    black_box(string);
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("rope_end", size), size, |b, _| {
            b.iter_batched(
                || Rope::from(text.as_str()),
                |mut rope| {
                    rope.insert(black_box(*size), black_box(insert_text));
                    black_box(rope);
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("ropey_end", size), size, |b, _| {
            b.iter_batched(
                || ropey::Rope::from(text.as_str()),
                |mut ropey| {
                    ropey.insert(black_box(*size), black_box(insert_text));
                    black_box(ropey);
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("string_end", size), size, |b, _| {
            b.iter_batched(
                || text.clone(),
                |mut string| {
                    string.push_str(black_box(insert_text));
                    black_box(string);
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

fn bench_delete_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete");

    for size in [1_000, 10_000, 100_000].iter() {
        let text = "a".repeat(*size);
        let delete_size = size / 10;

        group.throughput(Throughput::Elements(delete_size as u64));

        group.bench_with_input(BenchmarkId::new("rope_beginning", size), size, |b, _| {
            b.iter_batched(
                || Rope::from(text.as_str()),
                |mut rope| {
                    rope.delete(black_box(0..delete_size));
                    black_box(rope);
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("ropey_beginning", size), size, |b, _| {
            b.iter_batched(
                || ropey::Rope::from(text.as_str()),
                |mut ropey| {
                    ropey.remove(black_box(0..delete_size));
                    black_box(ropey);
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("string_beginning", size), size, |b, _| {
            b.iter_batched(
                || text.clone(),
                |mut string| {
                    string.replace_range(black_box(0..delete_size), "");
                    black_box(string);
                },
                criterion::BatchSize::SmallInput,
            )
        });

        let start = size / 2 - delete_size / 2;
        let end = size / 2 + delete_size / 2;
        group.bench_with_input(BenchmarkId::new("rope_middle", size), size, |b, _| {
            b.iter_batched(
                || Rope::from(text.as_str()),
                |mut rope| {
                    rope.delete(black_box(start..end));
                    black_box(rope);
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("ropey_middle", size), size, |b, _| {
            b.iter_batched(
                || ropey::Rope::from(text.as_str()),
                |mut rope| {
                    rope.remove(black_box(start..end));
                    black_box(rope);
                },
                criterion::BatchSize::SmallInput,
            )
        });

        group.bench_with_input(BenchmarkId::new("string_middle", size), size, |b, _| {
            b.iter_batched(
                || text.clone(),
                |mut string| {
                    string.replace_range(black_box(start..end), "");
                    black_box(string);
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

fn bench_slice_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("slice");

    for size in [10_000, 100_000].iter() {
        let text = "a".repeat(*size);
        let rope = Rope::from(text.as_str());
        let ropey = ropey::Rope::from_str(text.as_str());

        group.throughput(Throughput::Elements(*size as u64 / 4));

        let start = size / 4;
        let end = 3 * size / 4;

        group.bench_with_input(BenchmarkId::new("rope", size), &rope, |b, rope| {
            b.iter(|| {
                let slice = rope.slice(black_box(start..end));
                black_box(slice);
            })
        });

        group.bench_with_input(BenchmarkId::new("ropey", size), &ropey, |b, ropey| {
            b.iter(|| {
                let slice = ropey.slice(black_box(start..end));
                black_box(slice);
            })
        });

        group.bench_with_input(BenchmarkId::new("string", size), &text, |b, text| {
            b.iter(|| {
                let slice = &text[black_box(start..end)];
                let owned = slice.to_string();
                black_box(owned);
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_creation,
    bench_insert_operations,
    bench_delete_operations,
    bench_slice_operations
);
criterion_main!(benches);
