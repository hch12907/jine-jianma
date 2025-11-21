mod lsap;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Read, Write, stdout};
use std::path::{Path, PathBuf};

use argh::FromArgs;
use compact_str::CompactString;

#[derive(FromArgs)]
/// 简码计算
struct Args {
    #[argh(option, default=r#"String::from("beiyu:0.5,zhihu:0.5,tw:0.01")"#)]
    /// 字频表，可以混合使用多个字频表。默认为 beiyu:0.5,zhihu:0.5,tw:0.01。
    frequency: String,

    #[argh(option, default=r#"PathBuf::from("mabiao/yuming_chaifen.count.txt")"#)]
    /// 字根数量表，记录了一个字有多少个字根。
    count: PathBuf,

    #[argh(option, default=r#"PathBuf::from("mabiao/yuming_chaifen.allow.txt")"#)]
    /// 可用编码表，里头列出被额外允许的一些编码。
    /// 一行一个编码，# 开头的行会被忽略。
    allow: PathBuf,

    #[argh(option, default=r#"PathBuf::from("mabiao/yuming_chaifen.predefined.txt")"#)]
    /// 预定义编码表，里头列出被特别制定的编码。
    /// 一行一个编码与汉字（用 \t 隔开），# 开头的行会被忽略。
    predefined: PathBuf,

    #[argh(option, default=r#"PathBuf::from("output/yuming.txt")"#)]
    /// 简码表输出路径。
    out: PathBuf,

    #[argh(switch)]
    /// 打印候选简码。
    print_candidates: bool,

    #[argh(switch)]
    /// 允许空格简码。
    space_jianma: bool,

    #[argh(option, default=r#"String::from("aeiou")"#)]
    /// B区键位，默认为 aeiou。代码永远会假设空格是B区键位之一，因此不需要加入空格。
    b_area: String,

    #[argh(switch)]
    /// 按频率排序生成出来的简码表。
    sort_freq: bool,

    #[argh(positional, default=r#"PathBuf::from("mabiao/yuming_chaifen.dict.yaml")"#)]
    /// 宇浩拆分文件
    mabiao: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Character {
    bianma: CompactString,
    weight: u64,
    zigen_count: u64,
}

fn get_viable_mabiao(content: &str) -> HashMap<char, Character> {
    let mut result = HashMap::new();

    for line in content.lines() {
        if line.is_empty() {
            continue
        }

        let mut line = line.split('\t');
        let zi = line.next().unwrap();
        let info = line.next().unwrap()
            .strip_prefix('[')
            .unwrap()
            .strip_suffix(']')
            .unwrap();

        let bianma = info.split(',').nth(1).unwrap();
        let category = info.split(',').nth(4).unwrap();
        
        if category == "CJK" {
            let bianma = CompactString::from_str_to_lowercase(bianma);
            let character = Character {
                bianma,
                weight: 0,
                zigen_count: 0,
            };
            result.insert(zi.chars().next().unwrap(), character);
        }
    }

    result
}

fn initialize_weight(mabiao: &mut HashMap<char, Character>, frequency: &str) {
    let freq_files = frequency.split(',')
        .map(|file| {
            let mut file = file.split(':');
            let name = file.next().unwrap();
            let weight = file.next().and_then(|w| w.parse::<f64>().ok()).unwrap_or(1.0);
            (name, weight)
        })
        .collect::<Vec<_>>();
    
    for (freq_file, weight) in freq_files {
        let file = File::open(&format!("frequency/{freq_file}.json"))
            .expect(&format!("无法打开字频表 {freq_file}"));

        let json: HashMap<char, u64> = serde_json::from_reader(file)
            .expect(&format!("无法解析字频表JSON {freq_file}"));

        for (zi, freq) in json.iter() {
            let zi_freq = ((*freq as f64) * weight).round() as u64;

            if let Some(freq) = mabiao.get_mut(zi) {
                freq.weight += zi_freq;
            }
        }
    }

    mabiao.retain(|_zi, character| {
        character.weight > 1
    });
}

fn initialize_zigen_count(mabiao: &mut HashMap<char, Character>, path: &Path) {
    let mut content = String::new();

    if path.as_os_str().is_empty() {
        return;
    }

    File::open(path)
        .expect("无法打开字根数量表")
        .read_to_string(&mut content)
        .expect("无法读取字根数量表");

    for line in content.lines() {
        let line = line
            .split_once('#')
            .map(|(prefix, _suffix)| prefix)
            .unwrap_or(line)
            .trim_ascii();

        if line.is_empty() {
            continue
        }

        let mut line = line.split('\t');
        let zi = line.next().unwrap().chars().next().unwrap();
        let count = line.next().unwrap().parse::<u64>().unwrap();

        if let Some(zi) = mabiao.get_mut(&zi) {
            zi.zigen_count = count;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Predefined {
    bianma: CompactString,
    zi: CompactString,
}

fn read_allow_file(path: &Path) -> Vec<CompactString> {
    let mut content = String::new();

    if path.as_os_str().is_empty() {
        return Vec::new();
    }

    File::open(path)
        .expect("无法打开可用编码表")
        .read_to_string(&mut content)
        .expect("无法读取可用编码表");

    let mut result = Vec::new();

    for line in content.lines() {
        let line = line
            .split_once('#')
            .map(|(prefix, _suffix)| prefix)
            .unwrap_or(line)
            .trim_ascii();

        if line.is_empty() {
            continue
        }

        result.push(CompactString::from_str_to_lowercase(line))
    }

    result
}

fn read_predefined_file(path: &Path) -> Vec<Predefined> {
    let mut content = String::new();

    if path.as_os_str().is_empty() {
        return Vec::new();
    }

    File::open(path)
        .expect("无法打开预定义编码表")
        .read_to_string(&mut content)
        .expect("无法读取预定义编码表");

    let mut result = Vec::new();

    for (i, line) in content.lines().enumerate() {
        let line = line
            .split_once('#')
            .map(|(prefix, _suffix)| prefix)
            .unwrap_or(line)
            .trim_ascii();

        if line.is_empty() {
            continue
        }

        if let Some((prefix, suffix)) = line.split_once('\t') {
            let bianma = CompactString::from_str_to_lowercase(suffix);
            let zi = CompactString::new(prefix);
            result.push(Predefined { bianma, zi });
        } else {
            panic!("预定义编码表第{}行存在错误", i + 1);
        }
    }

    result
}

fn make_jianma_candidate(
    mabiao: &HashMap<char, Character>,
    allowed: &Vec<CompactString>,
    predefineds: &Vec<Predefined>,
) -> Vec<(char, Character)> {
    let mut result = Vec::new();
    let mut unavailable_bianma = mabiao
        .values()
        .map(|ch| ch.bianma.clone())
        .collect::<HashSet<_>>();
    unavailable_bianma.extend(predefineds.iter().map(|pre| pre.bianma.clone()));

    let allowed = allowed.iter().cloned().collect::<HashSet<_>>();
    let predefineds = predefineds
        .iter()
        .filter(|pred| pred.zi.chars().count() == 1)
        .map(|pred| pred.zi.chars().next().unwrap())
        .collect::<HashSet<_>>();

    for (zi, character) in mabiao.iter() {
        if predefineds.contains(zi) {
            continue
        }

        if character.bianma.len() < 3 {
            continue
        }

        for i in 1..character.bianma.len().clamp(3, 5) - 1 {
            let mut jianma = CompactString::from(&character.bianma.as_str()[..i]);
            jianma.push(character.bianma.chars().last().unwrap());

            if unavailable_bianma.contains(&jianma)
                && !allowed.contains(&jianma)
            {
                continue
            }

            let jianma_diff = (character.bianma.len().min(5) - jianma.len()) as f64;
            let jianma_weight = character.weight as f64
                // * jianma_diff
                * f64::powf(1.8, jianma_diff - 1.0)
                * (1.0 + 0.20 * (character.zigen_count.min(3) as f64 - 1.0))
                + 0.0;

            if jianma_weight > 8000.0 {
                result.push((*zi, Character {
                    bianma: CompactString::from(jianma),
                    weight: jianma_weight as u64,
                    zigen_count: character.zigen_count,
                }));
            }
        }
    }

    result
}

fn make_space_jianma_candidate(
    mabiao: &HashMap<char, Character>,
    suffix_jianma: &Vec<(char, Character)>,
    predefineds: &Vec<Predefined>,
    b_area: &[char],
) -> Vec<(char, Character)> {
    let mut result = Vec::new();
    let unneeded_zi = suffix_jianma
        .iter()
        .filter(|(_zi, ch)| ch.bianma.len() <= 3)
        .map(|(zi, _ch)| *zi)
        .chain(predefineds
            .iter()
            .filter(|pre| pre.bianma.len() <= 3 && pre.zi.chars().count() == 1)
            .map(|pre| pre.zi.chars().next().unwrap())
        )
        .collect::<HashSet<_>>();

    let unavailable_bianma = predefineds
        .iter()
        .filter(|pre| !pre.bianma.ends_with(b_area))
        .map(|pre| pre.bianma.clone())
        .collect::<HashSet<_>>();

    for (zi, character) in mabiao.iter() {
        if unneeded_zi.contains(zi) {
            continue
        }

        if character.bianma.len() < 3 {
            continue
        }

        for i in 1..(character.bianma.len() - 1).min(4) {
            let jianma = CompactString::from(&character.bianma.as_str()[..i]);

            if unavailable_bianma.contains(&jianma) {
                continue
            }

            let jianma_diff = (character.bianma.len().min(5) - jianma.len() - 1) as f64;
            let jianma_weight = character.weight as f64
                // * jianma_diff
                * f64::powf(1.8, jianma_diff - 1.0)
                * (1.0 + 0.30 * (character.zigen_count.min(3) as f64 - 1.0))
                * f64::powf(10.0, -(jianma.len().saturating_sub(2) as f64))
                + 0.0;

            if jianma_weight > 60000.0 {
                result.push((*zi, Character {
                    bianma: CompactString::from(jianma),
                    weight: jianma_weight as u64,
                    zigen_count: character.zigen_count,
                }));
            }
        }
    }

    result
}

fn write_jianma_candidate<W: Write>(writer: W, candidates: &Vec<(char, Character)>) {
    let mut candidates = candidates.iter()
        .map(|(zi, character)| {
            (*zi, character.bianma.clone(), character.weight)
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| a.2.cmp(&b.2).reverse());

    let mut writer = BufWriter::new(writer);

    for (zi, bianma, weight) in candidates.iter() {
        write!(writer, "{zi}\t{bianma}\t{weight}\n").unwrap();
    }
}

fn make_jianma_table_lsap(jianma: &Vec<(char, Character)>) -> (u64, Vec<(char, Character)>) {
    let zis = jianma
        .iter()
        .map(|(zi, _)| *zi)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let bianmas = jianma
        .iter()
        .map(|(_, ch)| ch.bianma.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let scores = jianma.iter().map(|(zi, ch)| ((zi, &ch.bianma), ch.weight)).collect::<HashMap<_, _>>();

    let mut cost_matrix = vec![0.0; zis.len() * bianmas.len()];
    for (i, bianma) in bianmas.iter().enumerate() {
        for (j, zi) in zis.iter().enumerate() {
            if let Some(score) = scores.get(&(zi, bianma)) {
                cost_matrix[i * zis.len() + j] = *score as f64;
            }
        }
    }

    let optimal = {
        lsap::solve(bianmas.len(), zis.len(), &cost_matrix, true).unwrap()
    };
    
    let mut selected_jianma = Vec::new();
    let mut total_score = 0;

    for (&i, &j) in optimal.0.iter().zip(optimal.1.iter()) {
        let i = i as usize;
        let j = j as usize;

        let score = cost_matrix[i * zis.len() + j];

        if score > 0.0 {
            selected_jianma.push((zis[j], Character {
                bianma: bianmas[i].clone(),
                weight: score as u64,
                zigen_count: 0,
            }));
            total_score += score as u64;
        }
    }

    (total_score, selected_jianma)
}

fn write_selected_jianma<W: Write>(
    writer: W,
    jianmas: &Vec<(char, Character)>,
    predefineds: &Vec<Predefined>,
    b_area: &[char],
    space_jianma: bool,
    sort_by_score: bool,
) {
    let mut writer = BufWriter::new(writer);

    let selected_jianma = if !sort_by_score {
        let mut jianmas = jianmas
            .iter()
            .map(|(zi, ch)| {
                let mut zi_str = CompactString::new("");
                zi_str.push(*zi);
                (zi_str, ch.bianma.clone())
            })
            .collect::<Vec<_>>();
        jianmas.extend(predefineds.iter().map(|pre| (pre.zi.clone(), pre.bianma.clone())));

        jianmas.sort_by(|a, b| {
            if !a.1.ends_with(b_area) && b.1.ends_with(b_area) {
                Ordering::Less
            } else if a.1.ends_with(b_area) && !b.1.ends_with(b_area) {
                Ordering::Greater
            } else {
                a.1.len().cmp(&b.1.len()).then(a.1.cmp(&b.1))
            }
        });

        jianmas
    } else {
        let mut jianmas = jianmas.clone();
        jianmas.sort_by(|a, b| {
            a.1.weight.cmp(&b.1.weight).reverse()
        });

        let jianmas = jianmas.into_iter()
            .map(|(zi, ch)| {
                let mut zi_str = CompactString::new("");
                zi_str.push(zi);
                (zi_str, ch.bianma.clone())
            })
            .chain(predefineds.iter().map(|pre| (pre.zi.clone(), pre.bianma.clone())))
            .collect();

        jianmas
    };

    for (zi, bianma) in selected_jianma.iter() {
        if !space_jianma && !bianma.ends_with(b_area) {
            continue
        }
        write!(writer, "{zi}\t{bianma}\n").unwrap();
    }
}

fn main() {
    let args = argh::from_env::<Args>();
    let b_area = args.b_area.chars().collect::<Vec<_>>();

    let mut mabiao = String::new();
    File::open(&args.mabiao)
        .expect("无法打开码表")
        .read_to_string(&mut mabiao)
        .expect("无法读取码表");

    let mut mabiao = get_viable_mabiao(&mabiao);
    initialize_weight(&mut mabiao, &args.frequency);
    initialize_zigen_count(&mut mabiao, &args.count);

    let alloweds = read_allow_file(&args.allow);
    let predefineds = read_predefined_file(&args.predefined);
    let candidates = make_jianma_candidate(&mabiao, &alloweds, &predefineds);

    if args.print_candidates {
        write_jianma_candidate(stdout(), &candidates);
        return;
    }

    let (score, mut selected_jianma) = make_jianma_table_lsap(&candidates);

    let score_space = if args.space_jianma {
        let candidates = make_space_jianma_candidate(
            &mabiao, &selected_jianma, &predefineds, &b_area
        );
        let (score, jianma) = make_jianma_table_lsap(&candidates);
        selected_jianma.extend_from_slice(&jianma);
        score
    } else {
        0
    };

    write_selected_jianma(
        File::create(&args.out).expect("无法创建简码表文件"),
        &selected_jianma,
        &predefineds,
        &b_area,
        args.space_jianma,
        args.sort_freq,
    );

    println!("最终简码得分：");
    println!("韵码简\t\t{} 分", score);
    if args.space_jianma {
        println!("韵码加空格简\t{} 分", score + score_space);
    }
}
