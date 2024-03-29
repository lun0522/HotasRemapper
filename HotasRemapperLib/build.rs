use protobuf_codegen::Codegen as ProtobufCodeGen;

fn main() {
    ProtobufCodeGen::new()
        .cargo_out_dir("protos")
        .include("src")
        .input("src/protos/input_remapping.proto")
        .input("src/protos/settings.proto")
        .run_from_script();
}
