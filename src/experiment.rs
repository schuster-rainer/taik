use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use serde_json::{Value, json};

use crate::{Result, record};

const TRAIN: [[&str; 3]; 3] = [
    ["  ACORN ", "acorn", " "],
    ["\tMAIZE", "ORE", "  "],
    ["  ROOT ", "ROOT", "\t "],
];
const HELD: [[&str; 3]; 6] = [
    [" SEED ", "seed", " "],
    ["   PEAR  ", "QUINCE", "\t"],
    ["\tD-19", "d-19", " LIME "],
    ["rock", "birch-7", "rock"],
    ["TEA", "BARK", "tea"],
    ["  ", "\t", " "],
];
const JSON_LIMIT: usize = 64 * 1024;

struct Sample {
    output: PathBuf,
    tags: String,
    cost: [u64; 2],
    elapsed_ns: u128,
}

struct Trial {
    binary: PathBuf,
    folder: PathBuf,
    observations: Vec<Value>,
}

impl Trial {
    fn execute(
        &mut self,
        name: &str,
        operation: &str,
        baseline: &Path,
        tags: &[&str],
    ) -> Result<Sample> {
        let output = self.folder.join(format!("{name}.run"));
        let start = Instant::now();
        let status = Command::new(&self.binary)
            .args([operation])
            .arg(baseline)
            .arg(&output)
            .args(tags)
            .stdout(std::process::Stdio::null())
            .status()?;
        let elapsed_ns = start.elapsed().as_nanos();
        if !status.success() {
            return Err(format!("{name}: Factory run failed with {status}").into());
        }
        let proof = record::read_regular(&output, record::MAX_RECORD_BYTES)?;
        let tags = proof
            .lines()
            .find_map(|line| line.strip_prefix("tags="))
            .ok_or("missing output tags")?
            .to_owned();
        let cost = proof
            .lines()
            .find_map(|line| line.strip_prefix("artifact_work="))
            .ok_or("missing work counts")?;
        let (bytes, items) = cost.split_once(", ").ok_or("invalid work counts")?;
        let cost = [
            bytes
                .strip_prefix("lowercase_bytes:")
                .ok_or("missing lowercase count")?
                .parse()?,
            items
                .strip_prefix("sort_items:")
                .ok_or("missing sort count")?
                .parse()?,
        ];
        self.observations.push(json!({
            "name": name, "path": output, "operation": operation, "source": baseline,
            "elapsed_ns": elapsed_ns, "work": cost, "tags": tags,
        }));
        println!("{name}: lower {}, sort {}", cost[0], cost[1]);
        Ok(Sample {
            output,
            tags,
            cost,
            elapsed_ns,
        })
    }
}

pub fn run(baseline: &Path, folder: &Path) -> Result<()> {
    record::read_accepted(baseline)?;
    std::fs::create_dir(folder)?;
    let mut trial = Trial {
        binary: std::env::current_exe()?,
        folder: folder.canonicalize()?,
        observations: Vec::new(),
    };
    let baseline = baseline.canonicalize()?;

    let setup_start = Instant::now();
    let first = trial.execute("ga-producer", "evolve", &baseline, &[])?;
    let enlarged = trial.execute(
        "ga-consumer",
        "extend",
        &first.output,
        &["  FIR ", " ASH ", "  "],
    )?;
    let returned = trial.execute("ga-feedback", "evolve", &enlarged.output, &[])?;
    if returned.tags != enlarged.tags || !returned.tags.split(',').any(|tag| tag == "fir") {
        return Err("feedback must preserve the added tag".into());
    }
    let alternative = trial.execute("learn-producer", "evolve", &first.output, &[])?;
    let choices = [&baseline, &first.output, &alternative.output];
    let mut training = [[0u64; 2]; 3];
    for (index, choice) in choices.iter().enumerate() {
        for (sample, tags) in TRAIN.iter().enumerate() {
            let result =
                trial.execute(&format!("train-{index}-{sample}"), "extend", choice, tags)?;
            for (total, cost) in training[index].iter_mut().zip(result.cost) {
                *total += cost;
            }
        }
    }
    let winner = (0..choices.len())
        .min_by_key(|index| training[*index])
        .ok_or("no procedure candidates")?;
    let selector = trial.folder.join("learned-selector.json");
    let policy =
        json!({"execute":"extend","baseline":choices[winner],"observed_training_costs":training});
    write_json(&selector, &policy)?;
    let policy: Value = serde_json::from_str(&record::read_regular(&selector, JSON_LIMIT)?)?;
    let selected = Path::new(
        policy["baseline"]
            .as_str()
            .ok_or("missing retained baseline")?,
    );
    let operation = policy["execute"]
        .as_str()
        .ok_or("missing retained operation")?;
    let setup_ns = setup_start.elapsed().as_nanos();

    let mut held = Vec::new();
    let (mut original, mut reuse, mut rederive) = (0, 0, 0);
    for (index, tags) in HELD.iter().enumerate() {
        let old = trial.execute(&format!("held-old-{index}"), "extend", &baseline, tags)?;
        let learned = trial.execute(&format!("held-learned-{index}"), operation, selected, tags)?;
        let fresh = trial.execute(&format!("fresh-1-{index}"), "evolve", &baseline, &[])?;
        let verified = trial.execute(&format!("fresh-2-{index}"), "evolve", &fresh.output, &[])?;
        let tested = trial.execute(
            &format!("fresh-use-{index}"),
            "extend",
            &verified.output,
            tags,
        )?;
        if old.tags != learned.tags || old.tags != tested.tags {
            return Err("held-out outputs differ".into());
        }
        let fresh_ns = fresh.elapsed_ns + verified.elapsed_ns + tested.elapsed_ns;
        original += old.elapsed_ns;
        reuse += learned.elapsed_ns;
        rederive += fresh_ns;
        held.push(json!({
            "original_ns":old.elapsed_ns,"reuse_ns":learned.elapsed_ns,"rediscover_ns":fresh_ns,
            "original_work":old.cost,"retained_work":learned.cost,"tags":old.tags,
        }));
    }
    write_json(
        &trial.folder.join("results.json"),
        &json!({
            "producer":first.output,"consumer":enlarged.output,"feedback":returned.output,"policy":selector,
            "setup_ns":setup_ns,"reused_total_ns":setup_ns + reuse,"rederived_total_ns":rederive,
            "original_total_ns":original,"held":held,"observations":trial.observations,
        }),
    )?;
    record::publish(
        &trial.folder.join("summary.run"),
        &format!(
            "decision=observed\nidentity=galaxy-intelligence\nintent=Test exchange, feedback and transfer with the Rust Factory\n\
         baseline={baseline:?}\nscope=three training inputs and six held-out inputs\n\
         checks=validated feedback retains fir; original, reused and rederived procedures agree on every held-out output\n\
         policy=choose from measured training work using the supplied selection rule\n\
         setup_ns={setup_ns}\nreuse_ns={reuse}\nrederive_ns={rederive}\noriginal_ns={original}\n\
         evidence=results.json, learned-selector.json and individual run records\n\
         limitations=external worker supplied the learning rule; runtime samples include process and file overhead\n\
         continuation=reflect on the measured outcomes before extending the experiment\nend\n"
        ),
    )?;
    Ok(())
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    record::publish_text(
        path,
        &format!("{}\n", serde_json::to_string_pretty(value)?),
        JSON_LIMIT,
    )
}
