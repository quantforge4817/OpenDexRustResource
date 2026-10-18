use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use opendex_user::auth::AuthService;
use uuid::Uuid;

/// Benchmark token creation via registration
fn bench_auth_token_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("auth");

    group.bench_function("register_and_create_token", |b| {
        b.iter_custom(|iters| {
            let mut time = std::time::Duration::ZERO;
            for i in 0..iters {
                let mut auth = AuthService::new();
                let start = std::time::Instant::now();
                let username = format!("user_{}", i);
                let _ = auth.register(username, "password123".to_string());
                time += start.elapsed();
            }
            time
        })
    });
}

/// Benchmark token validation (HMAC verification + expiration check)
fn bench_validate_token(c: &mut Criterion) {
    let mut group = c.benchmark_group("auth");

    // Pre-create a service with a registered user and token
    let mut auth = AuthService::new();
    let result = auth
        .register("benchmark_user".to_string(), "password123".to_string())
        .unwrap();
    let token_str = result.token.token.clone();

    group.bench_function("validate_token", |b| {
        b.iter(|| {
            auth.validate_token(&token_str);
        })
    });
}

/// Benchmark login flow (password verification + token creation)
fn bench_login(c: &mut Criterion) {
    let mut group = c.benchmark_group("auth");

    let mut auth = AuthService::new();
    auth.register("login_user".to_string(), "password123".to_string())
        .unwrap();

    group.bench_function("login_with_password", |b| {
        b.iter(|| {
            // Each iteration creates a new service to avoid rate limiting
            let mut auth = AuthService::new();
            auth.register("login_user".to_string(), "password123".to_string())
                .unwrap();
            let _ = auth.login("login_user", "password123");
        })
    });
}

/// Benchmark Argon2id password hashing (via registration)
fn bench_password_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("auth");

    group.bench_function("argon2id_hash_password", |b| {
        b.iter(|| {
            let mut auth = AuthService::new();
            let username = format!("hash_user_{}", Uuid::new_v4().hyphenated());
            let _ = auth.register(username, "complex_password_123!".to_string());
        })
    });
}

/// Benchmark password verification (via login)
fn bench_password_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("auth");

    group.bench_function("verify_password", |b| {
        b.iter(|| {
            let mut auth = AuthService::new();
            auth.register("verify_user".to_string(), "password123".to_string())
                .unwrap();
            let _ = auth.login("verify_user", "password123");
        })
    });
}

/// Benchmark token validation with increasing user counts
fn bench_validate_token_with_users(c: &mut Criterion) {
    let mut group = c.benchmark_group("auth");

    for &user_count in &[10, 100, 1_000] {
        group.bench_with_input(
            BenchmarkId::new("validate_token_with_users", user_count),
            &user_count,
            |b, &n| {
                let mut auth = AuthService::new();
                let mut target_token = String::new();
                for i in 0..n {
                    let username = format!("user_{}", i);
                    if i == n / 2 {
                        // Remember the middle user's token
                        let result = auth.register(username, "password123".to_string()).unwrap();
                        target_token = result.token.token.clone();
                    } else {
                        let _ = auth.register(username, "password123".to_string());
                    }
                }

                b.iter(|| {
                    auth.validate_token(&target_token);
                });
            },
        );
    }
}

criterion_group!(
    benches,
    bench_auth_token_creation,
    bench_validate_token,
    bench_login,
    bench_password_hashing,
    bench_password_verification,
    bench_validate_token_with_users,
);
criterion_main!(benches);
