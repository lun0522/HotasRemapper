use swift_rs::SwiftLinker;

fn main() {
    SwiftLinker::new("14.2")
        .with_package("HotasRemapperBt", "../HotasRemapperBt")
        .link();
}
