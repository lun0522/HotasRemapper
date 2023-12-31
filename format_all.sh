find HotasRemapper -iname '*.h' | xargs clang-format -i
swift-format -ir HotasRemapper HotasRemapperBt
rustfmt --edition=2021 HotasRemapperLib/src/**/*.rs
