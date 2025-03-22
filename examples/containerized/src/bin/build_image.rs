use std::fs::{self, File};
use std::io::Write;

use bollard::Docker;
use bollard::image::{BuildImageOptions, BuilderVersion};
use bollard::service::BuildInfoAux;
use daemon_slayer::core::server::tokio_stream::StreamExt;
use ignore::Walk;

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_local_defaults().unwrap();
    let dockerfile = fs::read_to_string("./Dockerfile").unwrap();

    let mut header = tar::Header::new_gnu();
    header.set_path("Dockerfile").unwrap();
    header.set_size(dockerfile.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    let mut tar = tar::Builder::new(Vec::new());

    // // tar.append_dir_all(path, src_path)
    tar.append(&header, dockerfile.as_bytes()).unwrap();
    for result in Walk::new("../..").flatten() {
        if result.path().is_dir() {
            tar.append_dir(
                result
                    .path()
                    .to_string_lossy()
                    .to_string()
                    .replace("../..", "."),
                result.path(),
            )
            .unwrap();
        } else {
            tar.append_file(
                result
                    .path()
                    .to_string_lossy()
                    .to_string()
                    .replace("../..", "."),
                &mut File::open(result.path()).unwrap(),
            )
            .unwrap();
        }
    }

    let uncompressed = tar.into_inner().unwrap();
    let mut c = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    c.write_all(&uncompressed).unwrap();
    let compressed = c.finish().unwrap();

    let build_image_options = BuildImageOptions {
        t: "myapp",
        dockerfile: "Dockerfile",
        version: BuilderVersion::BuilderBuildKit,
        pull: true,
        session: Some(String::from("myapp")),
        ..Default::default()
    };

    let mut image_build_stream =
        docker.build_image(build_image_options, None, Some(compressed.into()));

    while let Some(Ok(bollard::models::BuildInfo {
        aux: Some(BuildInfoAux::BuildKit(inner)),
        ..
    })) = image_build_stream.next().await
    {
        for vertex in inner.vertexes {
            println!("{} {}", vertex.name, vertex.digest);
        }
        for status in inner.statuses {
            println!(
                "{} {} {}",
                status.name,
                status.id,
                status.timestamp.map(|t| t.seconds).unwrap_or_default()
            );
        }

        for log in inner.logs {
            print!("{}", String::from_utf8(log.msg).unwrap());
        }
    }
}
