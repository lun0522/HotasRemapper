use swift_rs::SwiftLinker;
use protobuf_codegen::Codegen as ProtobufCodeGen;

fn main() {
    ProtobufCodeGen::new()
        .cargo_out_dir("protos")
        .include("src")
        .input("src/protos/input_remapping.proto")
        .run_from_script();
    SwiftLinker::new("14.2")
        .with_package("HotasRemapperBt", "../HotasRemapperBt")
        .link();
}
