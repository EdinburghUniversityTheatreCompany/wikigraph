mod walker;

use std::{fs, path::*};

use wikigraph_model::Manifest;

use crate::walker::Corpus;

#[derive(argh::FromArgs)]
#[argh(description = "scrapes wiki pages for wikigraph")]
pub(crate) struct Args {
    #[argh(option, short = 'u')]
    #[argh(description = "root url for reintegrating external links")]
    pub url_root: Option<String>,
    #[argh(positional)]
    pub input_path: PathBuf,
    #[argh(positional)]
    pub output_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .init();

    let args: Args = argh::from_env();

    let corpus = Corpus::from_args(&args)?;
    tracing::info!("compiling corpus from {:?}...", args.input_path);
    let mut graph = corpus.walk()?;
    tracing::info!("corpus compiled!");

    let mut analysis = wikigraph_model::analysis::AnalysisGraph::new(&graph);
    analysis.calculate_stats(&mut graph);
    graph.scc = analysis.export_scc();
    graph.shortest_paths = analysis.export_shortest_paths();

    let json = serde_json::to_string(&Manifest {
        time: jiff::Zoned::now(),
        graph,
    })?;
    fs::write(args.output_path, &json)?;

    Ok(())
}
