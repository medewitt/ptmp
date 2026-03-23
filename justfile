build:
  cargo build --release

doc:
  cargo doc --no-deps --document-private-items --open

install:
  cargo install --path .