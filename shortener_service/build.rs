// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     tonic_build::compile_protos("../proto/hello.proto")?;
//     Ok(())
// }
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../proto/url.proto")?;
    Ok(())
}
