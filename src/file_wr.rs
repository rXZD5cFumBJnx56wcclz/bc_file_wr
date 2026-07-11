use std::borrow::Borrow;
use std::error::Error;
use std::fs::{self, File, copy, create_dir_all, remove_dir_all};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use bc_utils::other::transpose;
use bc_utils_lg::structs::settings::SETTINGS_FILES_PATH;
use bc_utils_lg::types::maps::{MAP, MAP_LINK, MapTrait};
use bincode::config::standard;
use bincode::serde::{decode_from_slice, encode_to_vec};

use bc_visual::visual::{BT_SCRIPT, STAT_COLUMNS_SCRIPT, STAT_VALUES_SCRIPT};

pub fn get_backtest_dir(
    dir: &str,
    symbol: &str,
    time: u64,
) -> String {
    format!("{dir}/{time}/{symbol}",)
}

fn write_any_data_column<'a, T, M>(
    path: &str,
    file_path: &str,
    data: &'a [T],
) -> std::io::Result<()>
where
    T: Borrow<M>,
    M: MapTrait<'a, &'a str, Vec<f64>>,
    M: 'a,
{
    create_dir_all(path)?;
    let mut buf = BufWriter::new(File::create_new(file_path)?);
    for el in data {
        writeln!(
            buf,
            "{}",
            el.borrow()
                .keys()
                .into_iter()
                .map(|v| *v)
                .collect::<Vec<&str>>()
                .join(" ")
        )?;
        if !el.borrow().is_empty() {
            for i in 0..el
                .borrow()
                .values()
                .into_iter()
                .next()
                .unwrap_or(&vec![])
                .len()
            {
                writeln!(
                    buf,
                    "{}",
                    el.borrow()
                        .values()
                        .into_iter()
                        .map(|v| v[i].to_string())
                        .collect::<Vec<String>>()
                        .join(" ")
                )?;
            }
            writeln!(buf, "\n\n")?;
        }
    }

    Ok(())
}
fn write_any_data_value(
    path: &str,
    file_path: &str,
    data: &MAP<&str, f64>,
) -> std::io::Result<()> {
    create_dir_all(path)?;
    let mut buf = BufWriter::new(File::create_new(file_path)?);
    for (k, v) in data {
        writeln!(buf, "{k} {v}",)?;
    }
    Ok(())
}

fn parse_data_columns<'a>(
    splitted: impl IntoIterator<Item = &'a str>
) -> Result<Vec<MAP<String, Vec<f64>>>, Box<dyn Error>> {
    splitted
        .into_iter()
        .filter(|v| !v.is_empty() && *v != "\n")
        .map(|data| -> Result<MAP<String, Vec<f64>>, Box<dyn Error>> {
            transpose(
                data.split("\n")
                    .into_iter()
                    .map(|v| v.split(" ").collect())
                    .collect(),
            )
            .into_iter()
            .map(|v| -> Result<(String, Vec<f64>), Box<dyn Error>> {
                Ok((
                    v[0].to_string(),
                    v.into_iter()
                        .skip(1)
                        .map(|f| -> Result<f64, Box<dyn Error>> {
                            dbg!(f);
                            Ok(f.parse::<f64>()?)
                        })
                        .collect::<Result<Vec<f64>, Box<dyn Error>>>()?,
                ))
            })
            .collect::<Result<MAP<String, Vec<f64>>, Box<dyn Error>>>()
        })
        .collect::<Result<Vec<MAP<String, Vec<f64>>>, Box<dyn Error>>>()
}

fn parse_data_values<'a>(
    splitted: impl Iterator<Item = &'a str>
) -> Result<Vec<MAP<String, f64>>, Box<dyn Error>> {
    splitted
        .into_iter()
        .map(|v| {
            v.split("\n")
                .filter(|v1| !v1.is_empty() && *v1 != "\n")
                .map(|v2| -> Result<(String, f64), Box<dyn Error>> {
                    let mut sp = v2.split(" ");
                    Ok((
                        sp.next().ok_or("err")?.to_string(),
                        sp.next().ok_or("err")?.parse::<f64>()?,
                    ))
                })
                .collect::<Result<MAP<String, f64>, Box<dyn Error>>>()
        })
        .collect()
}

pub struct FileWR<'a> {
    s: &'a SETTINGS_FILES_PATH,
}

impl<'a> FileWR<'a> {
    pub fn new(s: &'a SETTINGS_FILES_PATH) -> Self {
        Self { s }
    }
}

impl FileWR<'_> {
    pub fn src_write(
        &self,
        src: &Vec<Vec<f64>>,
    ) -> Result<(), Box<dyn Error>> {
        if !self.s.src.is_file() {
            create_dir_all(&self.s.src)?;
            fs::write(
                &format!("{}/src.bin", self.s.src.to_str().unwrap()),
                encode_to_vec(src, standard())?,
            )?;
        }
        Ok(())
    }
    pub fn src(&self) -> Result<Vec<Vec<f64>>, Box<dyn Error>> {
        Ok(decode_from_slice(
            &fs::read(&format!("{}/src.bin", self.s.src.to_str().ok_or("err")?))?,
            standard(),
        )?
        .0)
    }
    pub fn src_or(
        &self,
        or: Vec<Vec<f64>>,
    ) -> Vec<Vec<f64>> {
        self.src().unwrap_or(or)
    }
    pub fn src_symbols_write(
        &self,
        src_symbols: &MAP<String, Vec<Vec<f64>>>,
    ) -> Result<(), Box<dyn Error>> {
        if !self.s.src.is_file() {
            create_dir_all(&self.s.src)?;
            fs::write(
                &format!("{}/src_symbols.bin", self.s.src.to_str().unwrap()),
                encode_to_vec(src_symbols, standard())?,
            )?;
        }
        Ok(())
    }
    pub fn src_symbols(&self) -> Result<MAP<String, Vec<Vec<f64>>>, Box<dyn Error>> {
        Ok(decode_from_slice(
            &fs::read(&format!(
                "{}/src_symbols.bin",
                self.s.src.to_str().ok_or("err")?
            ))?,
            standard(),
        )?
        .0)
    }
    pub fn src_symbols_or(
        &self,
        or: MAP<String, Vec<Vec<f64>>>,
    ) -> MAP<String, Vec<Vec<f64>>> {
        self.src_symbols().unwrap_or(or)
    }
    pub fn script_write(
        &self,
        dir: &str,
        file_name: &str,
        script: &str,
    ) -> Result<(), Box<dyn Error>> {
        create_dir_all(dir)?;
        let path = format!("{dir}/{}", file_name);
        if !self.s.script_backtest.exists() {
            let mut file = File::create_new(&path)?;
            writeln!(file, "{}", script)?;
        } else {
            copy(self.s.script_backtest.to_str().unwrap(), path)?;
        }
        Ok(())
    }
    pub fn script(
        &self,
        path: &PathBuf,
    ) -> Result<String, Box<dyn Error>> {
        Ok(fs::read_to_string(path)?)
    }
    pub fn script_or(
        &self,
        path: &PathBuf,
        script: String,
    ) -> String {
        self.script(path).unwrap_or(script)
    }
    pub fn backtest_write(
        &self,
        data: &Vec<MAP_LINK<&str, Vec<f64>>>,
        stat_columns: &MAP<&str, Vec<f64>>,
        stat_values: &MAP<&str, f64>,
        symbol: &str,
        time: u64,
    ) -> Result<(), Box<dyn Error>> {
        let dir = get_backtest_dir(self.s.backtest.to_str().unwrap(), symbol, time);
        create_dir_all(&dir)?;
        if self.s.backtest.is_dir() {
            self.script_write(&dir, "script_backtest.plt", &BT_SCRIPT(symbol))?;
            self.script_write(
                &dir,
                "script_stat_columns.plt",
                &STAT_COLUMNS_SCRIPT(symbol),
            )?;
            self.script_write(&dir, "script_stat_values.plt", &STAT_VALUES_SCRIPT(symbol))?;
            write_any_data_column(&dir, &format!("{dir}/data.dat"), data)?;
            write_any_data_column::<&MAP<_, _>, MAP<_, _>>(
                &dir,
                &format!("{dir}/stat_columns.dat"),
                &[stat_columns],
            )?;
            write_any_data_value(&dir, &format!("{dir}/stat_values.dat",), stat_values)?;
        }
        Ok(())
    }
    pub fn backtest(
        &self,
        dir: &PathBuf,
    ) -> Result<
        (
            Vec<MAP<String, Vec<f64>>>,
            MAP<String, Vec<f64>>,
            MAP<String, f64>,
        ),
        Box<dyn Error>,
    > {
        Ok((
            parse_data_columns(
                fs::read_to_string(format!("{}/data.dat", dir.to_str().unwrap()))?.split("\n\n"),
            )?,
            {
                let mut bind = parse_data_columns(
                    [fs::read_to_string(format!("{}/stat_columns.dat", dir.to_str().unwrap()))?
                        .as_str()]
                    .into_iter(),
                )?;
                if bind.is_empty() {
                    Default::default()
                } else {
                    bind.remove(0)
                }
            },
            {
                let mut bind = parse_data_values(
                    [fs::read_to_string(format!("{}/stat_values.dat", dir.to_str().unwrap()))?
                        .as_str()]
                    .into_iter(),
                )?;
                if bind.is_empty() {
                    Default::default()
                } else {
                    bind.remove(0)
                }
            },
        ))
    }
    pub fn backtest_or(
        &self,
        dir: &PathBuf,
        or: (
            Vec<MAP<String, Vec<f64>>>,
            MAP<String, Vec<f64>>,
            MAP<String, f64>,
        ),
    ) -> (
        Vec<MAP<String, Vec<f64>>>,
        MAP<String, Vec<f64>>,
        MAP<String, f64>,
    ) {
        self.backtest(dir).unwrap_or(or)
    }
    // pub fn backtests_write(
    //     &self,
    //     data: &MAP<String, Vec<MAP_LINK<&str, Vec<f64>>>>,
    //     stat_columns: &<MAP<&str, Vec<f64>>>,
    //     stat_values: &MAP<&str, f64>,
    //     symbol: &str,
    //     time: u64,
    // )
    // train_model_write
    // train_model
    // train_model_or
}

impl FileWR<'_> {
    pub fn clear(&self) -> Result<(), Box<dyn Error>> {
        let path = Path::new("target/bc_constructor");
        if path.exists() {
            return Ok(remove_dir_all(&path)?);
        }
        Ok(())
    }
    pub fn backtests_del(&self) -> Result<(), Box<dyn Error>> {
        if self.s.backtest.exists() {
            return Ok(remove_dir_all(&self.s.backtest)?);
        }
        Ok(())
    }
    pub fn src_del(&self) -> Result<(), Box<dyn Error>> {
        if self.s.src.exists() {
            return Ok(remove_dir_all(&self.s.src)?);
        }
        Ok(())
    }
    pub fn script_backtest_del(&self) -> Result<(), Box<dyn Error>> {
        if self.s.script_backtest.exists() {
            return Ok(remove_dir_all(&self.s.script_backtest)?);
        }
        Ok(())
    }
    pub fn script_stat_del(&self) -> Result<(), Box<dyn Error>> {
        if self.s.script_stat.exists() {
            return Ok(remove_dir_all(&self.s.script_stat)?);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::{
        fs::remove_dir_all,
        path::Path,
        pin::Pin,
        sync::{LazyLock, Mutex},
    };

    use bc_trade_simulate::statistics::{StatCollector, StatData};
    use bc_trade_simulate::trade_data::AfterTradeData;
    use bc_utils_lg::statics::prices::*;
    use bc_utils_lg::structs::settings::SETTINGS;
    use bc_utils_lg::structs::trade::TradeCell;

    static S: LazyLock<SETTINGS_FILES_PATH> = LazyLock::new(|| SETTINGS_FILES_PATH {
        backtest: "test_dir/backtest".into(),
        src: "test_dir/src".into(),
        train_model: "test_dir/train_model".into(),
        ..Default::default()
    });

    static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
    static S_DF: LazyLock<SETTINGS> = LazyLock::new(|| Default::default());
    static F: LazyLock<FileWR> = LazyLock::new(|| FileWR::new(&S));
    static STAT_DATA_AFTER_DATA: LazyLock<
        fn() -> (StatData<'static>, Pin<Box<AfterTradeData<'static>>>),
    > = LazyLock::new(|| {
        || {
            let mut stat_collector = StatCollector::new("".to_string(), &S_DF.trade);
            stat_collector.push(
                TradeCell::new(100., SRC_EL.clone(), SRC_EL1.clone()),
                Default::default(),
                Default::default(),
                Default::default(),
            );
            stat_collector.push(
                TradeCell::new(100., SRC_EL.clone(), SRC_EL1.clone()),
                Default::default(),
                Default::default(),
                Default::default(),
            );
            let stat_data = stat_collector.to_data();
            let stat_data_vec = stat_data.to_vec();
            let after_data = AfterTradeData::new(&S_DF, &stat_data_vec[0], &Default::default());
            (stat_data, after_data)
        }
    });

    fn remove_dir_all_(dir: &str) -> Result<(), Box<dyn Error>> {
        if Path::new(dir).exists() {
            Ok(remove_dir_all(dir)?)
        } else {
            Ok(())
        }
    }

    #[test]
    fn src_write_res_1() -> Result<(), Box<dyn Error>> {
        let _l = LOCK.lock()?;
        remove_dir_all_("test_dir")?;
        assert!(!S.src.exists());
        F.src_write(&SRC)?;
        assert!(Path::new(&format!("{}/src.bin", S.src.to_str().unwrap())).exists());
        remove_dir_all(&S.src)?;
        assert!(!S.src.exists());
        remove_dir_all_("test_dir")?;
        Ok(())
    }

    #[test]
    fn src_res_1() -> Result<(), Box<dyn Error>> {
        let _l = LOCK.lock()?;
        remove_dir_all_("test_dir")?;
        F.src_write(&SRC)?;
        assert!(Path::new(&format!("{}/src.bin", S.src.to_str().unwrap())).exists());
        let _: Vec<Vec<f64>> = F.src()?;
        remove_dir_all(&S.src)?;
        assert!(!S.src.exists());
        remove_dir_all_("test_dir")?;
        Ok(())
    }

    #[test]
    fn src_symbols_write_res_1() -> Result<(), Box<dyn Error>> {
        let _l = LOCK.lock()?;
        remove_dir_all_("test_dir")?;
        assert!(!S.src.exists());
        F.src_symbols_write(&MAP::default())?;
        assert!(Path::new(&format!("{}/src_symbols.bin", S.src.to_str().unwrap())).exists());
        remove_dir_all(&S.src)?;
        assert!(!S.src.exists());
        remove_dir_all_("test_dir")?;
        Ok(())
    }

    #[test]
    fn src_symbols_res_1() -> Result<(), Box<dyn Error>> {
        let _l = LOCK.lock()?;
        remove_dir_all_("test_dir")?;
        F.src_symbols_write(&MAP::default())?;
        assert!(Path::new(&format!("{}/src_symbols.bin", S.src.to_str().unwrap())).exists());
        let _: MAP<String, Vec<Vec<f64>>> = F.src_symbols()?;
        remove_dir_all(&S.src)?;
        assert!(!S.src.exists());
        remove_dir_all_("test_dir")?;
        Ok(())
    }

    #[test]
    fn script_write_res_1() -> Result<(), Box<dyn Error>> {
        let _l = LOCK.lock()?;
        remove_dir_all_("test_dir")?;
        let dir = get_backtest_dir("test_dir/backtest", "symbol", 1);
        let file_name = "script_backtest.plt";
        let bind = format!("{dir}/{file_name}",);
        let path = Path::new(&bind);
        assert!(!path.exists());
        F.script_write(&dir, file_name, &BT_SCRIPT("symbol"))?;
        assert!(path.exists());
        remove_dir_all(&dir)?;
        assert!(!path.exists());
        remove_dir_all("test_dir")?;
        Ok(())
    }

    #[test]
    fn script_res_1() -> Result<(), Box<dyn Error>> {
        let _l = LOCK.lock()?;
        remove_dir_all_("test_dir")?;
        let dir = get_backtest_dir("test_dir/backtest", "symbol", 1);
        let file_name = "script_backtest.plt";
        let bind = format!("{dir}/{file_name}",);
        let path = Path::new(&bind);
        F.script_write(&dir, file_name, &BT_SCRIPT("symbol"))?;
        assert!(path.exists());
        let _: String = F.script(&path.into())?;
        remove_dir_all(&dir)?;
        assert!(!path.exists());
        remove_dir_all_("test_dir")?;
        Ok(())
    }

    #[test]
    fn backtest_write_res_1() -> Result<(), Box<dyn Error>> {
        let _l = LOCK.lock()?;
        remove_dir_all_("test_dir")?;
        assert!(!S.backtest.exists());
        let (stat_data, after_data) = STAT_DATA_AFTER_DATA();
        let stat_data_vec = stat_data.to_vec();
        F.backtest_write(
            &stat_data,
            &after_data.to_stat_columns(&stat_data_vec[0]),
            &after_data.to_stat_values(&stat_data_vec[0]),
            "symbol",
            1,
        )?;
        assert!(Path::new(&format!("{}/1/symbol", S.backtest.to_str().unwrap())).exists());
        assert!(
            Path::new(&format!(
                "{}/1/symbol/script_backtest.plt",
                S.backtest.to_str().unwrap()
            ))
            .exists()
        );
        assert!(
            Path::new(&format!(
                "{}/1/symbol/script_stat_columns.plt",
                S.backtest.to_str().unwrap()
            ))
            .exists()
        );
        assert!(
            Path::new(&format!(
                "{}/1/symbol/script_stat_values.plt",
                S.backtest.to_str().unwrap()
            ))
            .exists()
        );
        remove_dir_all(&S.backtest)?;
        assert!(!S.backtest.exists());
        remove_dir_all_("test_dir")?;
        Ok(())
    }

    #[test]
    fn backtest_res_1() -> Result<(), Box<dyn Error>> {
        let _l = LOCK.lock()?;
        remove_dir_all_("test_dir")?;
        assert!(!S.backtest.exists());
        let (stat_data, after_data) = STAT_DATA_AFTER_DATA();
        let stat_data_vec = stat_data.to_vec();
        F.backtest_write(
            &stat_data,
            &after_data.to_stat_columns(&stat_data_vec[0]),
            &after_data.to_stat_values(&stat_data_vec[0]),
            "symbol",
            1,
        )?;
        let _: (
            Vec<MAP<String, Vec<f64>>>,
            MAP<String, Vec<f64>>,
            MAP<String, f64>,
        ) = F.backtest(&format!("{}/1/symbol", S.backtest.to_str().unwrap()).into())?;
        remove_dir_all(&S.backtest)?;
        assert!(!S.backtest.exists());
        remove_dir_all_("test_dir")?;
        Ok(())
    }
}
