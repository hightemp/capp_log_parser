
use structopt::StructOpt;
use structopt::clap::arg_enum;

use std::option::Option;

use serde_json::{Value};
use serde::{Deserialize, Serialize};

use std::path::PathBuf;
use std::fs::read_to_string;
use std::fs::File;
use std::fs::metadata;
use std::io;
use std::io::{BufRead};

// Сделать генератор для парсинга параметров(модуль)

#[derive(Serialize, Deserialize, Debug)]
struct JSONResponse {
	iItemsCount: u32,
	iPageIndex: u32,
	iPageSize: u32,
	iPagesCount: u32,
	iFileSize: u64,
	aLines: Vec<String>
}

arg_enum! {
    #[derive(Debug)]
    enum FormatTypes {
		LinesNumber,
        Json,
        StringList
    }
}

#[derive(StructOpt)]
#[structopt(name = "capp_log_parser", about = "Парсер логов")]
struct Args {
    #[structopt(short = "s", long = "config-as-string", default_value = "")]
	sConfigAsString: String,

    #[structopt(short = "f", long = "filter", default_value = "")]
	sFilter: String,

    #[structopt(short = "c", long = "config-file-path", parse(from_os_str), default_value = "./capp_log_parser_config.json")]
	oConfigFilePath: PathBuf,

    #[structopt(short = "p", long = "page-index", default_value = "1")]
	iPageIndex: u32,
	
    #[structopt(short = "m", long = "page-size", default_value = "10")]
	iPageSize: u32,

	#[structopt(short = "t", default_value = "StringList", possible_values = &FormatTypes::variants(), case_insensitive = true)]
    eFormatType: FormatTypes,

	#[structopt(parse(from_os_str))]
    oParseFilePath: PathBuf,
}

fn fnGetLinesCount(oArgs: Args) -> String {
	let (iCount, _) = fnReadLines(&oArgs.oParseFilePath, oArgs.sFilter, 0, 0, false, true);

	return iCount.to_string();
}

fn fnGetLinesAsJson(oArgs: Args) -> String {
	let oConfig = fnLoadConfig(oArgs.oConfigFilePath);

	let oFilePath = oArgs.oParseFilePath;

	let (iCount, oVec) = fnReadLines(&oFilePath, oArgs.sFilter, oArgs.iPageIndex, oArgs.iPageSize, true, true);

	let sFileName = oFilePath.as_path();
	let oMetadata = metadata(sFileName).unwrap();

	let oJSONResponse = JSONResponse {
		iItemsCount: iCount,
		iPageIndex: oArgs.iPageIndex,
		iPageSize: oArgs.iPageSize,
		iPagesCount: ((iCount as f32/oArgs.iPageSize as f32) as f32).ceil() as u32,
		iFileSize: oMetadata.len(),
		aLines: oVec
	};

	return serde_json::to_string(&oJSONResponse).unwrap();
}

fn fnGetLinesAsStringList(oArgs: Args) -> String {
	let (_, oVec) = fnReadLines(&oArgs.oParseFilePath, oArgs.sFilter, oArgs.iPageIndex, oArgs.iPageSize, true, false);

	return oVec.join("\n");
}

fn fnReadLines(oFilePath: &PathBuf, sFilter: String, iPageIndex: u32, iPageSize: u32, bReturnLines: bool, bReturnCount: bool) -> (u32, Vec<String>) {
	let sFileName = oFilePath.as_path();

	let file = File::open(&sFileName).expect("Unable to open");
	let mut reader = io::BufReader::new(&file);
	let mut aResult = Vec::new();

	let mut lines = reader.lines();
	let mut iCount: u32 = 0;

	if iPageSize>0 {
		let iBeginIndex = (iPageIndex-1)*iPageSize;
		let mut iEndIndex = (iPageIndex)*iPageSize;

		let mut iIndex = 0;
		let mut sLine = String::new();

		loop {
			let oLine = lines.next();

			if oLine.is_none() {
				break;
			}

			sLine = oLine.unwrap().unwrap();

			if !sFilter.is_empty() && !sLine.contains(&sFilter) {
				continue;
			}

			if bReturnCount {
				iCount += 1;
			}

			if iIndex>=iBeginIndex && iIndex<iEndIndex {
				aResult.push(String::from(sLine.trim()));
			}

			iIndex += 1;
		}
	}

	return (iCount, aResult);
}

fn fnLoadConfig(oConfigFilePath: PathBuf) -> Value {
	let mut oConfig: Value = Value::Null;

	if oConfigFilePath.is_file() {
		let sConfigFilePath = oConfigFilePath.as_path();
		let sConfigBuffer = read_to_string(sConfigFilePath).unwrap();

		oConfig = serde_json::from_str(&sConfigBuffer).unwrap();
	} else {
		oConfig = serde_json::from_str(&"{}").unwrap();
	}

	return oConfig;
}

fn main() {
	let oArgs = Args::from_args();

	let sOutput = match oArgs.eFormatType {
		FormatTypes::LinesNumber => fnGetLinesCount(oArgs),
		FormatTypes::Json => fnGetLinesAsJson(oArgs),
		FormatTypes::StringList => fnGetLinesAsStringList(oArgs)
	};

	println!("{}", sOutput);
}
