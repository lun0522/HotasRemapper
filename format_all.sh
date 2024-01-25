find HotasRemapper -iname '*.h' | xargs clang-format -i
find HotasRemapperLib -iname '*.proto' | xargs clang-format -i
swift-format -ir HotasRemapper
rustfmt --edition=2021 HotasRemapperLib/src/**/*.rs
