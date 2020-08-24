
use structopt::StructOpt;
use structopt::clap::arg_enum;

use std::option::Option;

use serde_json::*;
use serde::{Deserialize, Serialize};

use std::path::PathBuf;
use std::fs::read_to_string;
use std::fs::File;
use std::fs::metadata;
use std::io;
use std::io::{BufRead};

use std::collections::HashMap;

use regex::*;

use math::round::ceil;

// use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime};
// use chrono::format::ParseError;
use chrono::{NaiveDateTime};

// Сделать генератор для парсинга параметров(модуль)

type StringHashMap = HashMap<String, String>;
type VecOfStringHashMap = Vec<StringHashMap>;

type VecOfStrings = Vec<String>;

#[derive(Serialize, Deserialize, Debug)]
struct JSONResponse {
	iItemsCount: u32,
	iPageIndex: u32,
	iPageSize: u32,
	iPagesCount: u32,
	iFileSize: u64,
	aLines: VecOfStringHashMap
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
    #[structopt(short = "s", long = "config-as-string", default_value = "", help = "json string with config")]
	sConfigAsString: String,

    #[structopt(short = "f", long = "filter", default_value = "", help = "filter by this value")]
	sFilter: String,

    #[structopt(short = "d", long = "date", default_value = "", help = "filter by date with format: dd.mm.YY:dd.mm.YY")]
	sDateFilter: String,

    #[structopt(short = "c", long = "config-file-path", parse(from_os_str), default_value = "./capp_log_parser_config.json", help = "path to json config")]
	oConfigFilePath: PathBuf,

    #[structopt(short = "p", long = "page-index", default_value = "1", help = "page number")]
	iPageIndex: u32,

    #[structopt(short = "l", long = "last-page", help = "show last page")]
	bShowLastPage: bool,

    #[structopt(short = "m", long = "page-size", default_value = "10", help = "items on page")]
	iPageSize: u32,

	#[structopt(short = "t", default_value = "StringList", possible_values = &FormatTypes::variants(), case_insensitive = true, help = "format type")]
    eFormatType: FormatTypes,

	#[structopt(parse(from_os_str), help = "path to log file")]
    oParseFilePath: PathBuf,
}

struct LogParser {
	oConfig: Value,
	sConfigAsString: String,
	oConfigFilePath: PathBuf,
	oFilePath: PathBuf, 
	sRegExp: String, 
	sDateFilter: String, 
	sFilter: String, 
	iPageIndex: u32, 
	bShowLastPage: bool,
	iPageSize: u32, 
	bReturnLines: bool, 
	bReturnCount: bool,
	iLinesCount: u32,
	aParsedList: VecOfStringHashMap,
	aStringList: VecOfStrings,

	eFormatType: FormatTypes,

	oTypes: Value,
	oFilesToTypes: Value,	
	sFileName: String,
	oFileBlock: Value
}

impl LogParser {
	fn fnParse(&mut self) -> String {
		self.fnLoadConfig();

		return match self.eFormatType {
			FormatTypes::LinesNumber => self.fnGetLinesCount(),
			FormatTypes::Json => self.fnGetLinesAsJson(),
			FormatTypes::StringList => self.fnGetLinesAsStringList()
		};	
	}

	fn fnGetLinesCount(&mut self) -> String {
		self.iPageIndex = 0;
		self.iPageSize = 0;
		self.bReturnCount = true;
		self.bReturnLines = false;

		self.fnReadLines();
	
		return self.iLinesCount.to_string();
	}
	
	fn fnGetLinesAsJson(&mut self) -> String {
		// let oConfig = fnLoadConfig();

		self.bReturnCount = true;
		self.bReturnLines = true;

		self.fnReadLines();
	
		let sFileName = self.oFilePath.as_path();
		let oMetadata = metadata(sFileName).unwrap();
	
		let oJSONResponse = JSONResponse {
			iItemsCount: self.iLinesCount,
			iPageIndex: self.iPageIndex,
			iPageSize: self.iPageSize,
			iPagesCount: ((self.iLinesCount as f32/self.iPageSize as f32) as f32).ceil() as u32,
			iFileSize: oMetadata.len(),
			aLines: self.aParsedList.clone()
		};
	
		return serde_json::to_string(&oJSONResponse).unwrap();
	}
	
	fn fnGetLinesAsStringList(&mut self) -> String {

		self.bReturnCount = false;
		self.bReturnLines = true;

		self.fnReadLines();
	
		return self.aStringList.join("\n");
	}

	fn fnParseDateFilter(&mut self) -> (Option<NaiveDateTime>, Option<NaiveDateTime>) {
		let mut oDateFrom: Option<NaiveDateTime> = None;
		let mut oDateTo: Option<NaiveDateTime> = None;
	
		if !self.sDateFilter.is_empty() {
			let aSplitted: Vec<&str> = self.sDateFilter.split('@').collect();
	
			let sDateFrom: String = String::from(match aSplitted.len() {
				1 => aSplitted[0],
				2 => aSplitted[0],
				_ => ""
			});
	
			let sDateTo: String = String::from(match aSplitted.len() {
				1 => aSplitted[0],
				2 => aSplitted[1],
				_ => ""
			});
			
			let sDatetimeFormat = "%Y-%m-%d_%H:%M:%S";
			let sZeroTime = String::from("00:00:00");
			
			let mut oDateFromResult = NaiveDateTime::parse_from_str(sDateFrom.as_str(), sDatetimeFormat);
			oDateFrom = match oDateFromResult {
				Err(sV) => Some(NaiveDateTime::parse_from_str([sDateFrom, sZeroTime.clone()].join("_").as_str(), sDatetimeFormat).unwrap()),
				Ok(oValue) => Some(oValue)
			};
			
			let mut oDateToResult = NaiveDateTime::parse_from_str(sDateTo.as_str(), sDatetimeFormat);
			oDateTo = match oDateToResult {
				Err(sV) => Some(NaiveDateTime::parse_from_str([sDateTo, sZeroTime.clone()].join("_").as_str(), sDatetimeFormat).unwrap()),
				Ok(oValue) => Some(oValue)
			};
		}

		return (oDateFrom, oDateTo);
	}

	// Загрузка блока из конфига
	fn fnLoadBlockFromConfig(&mut self) {
		if !self.oConfig.is_object() {
			return;
		}

		self.oTypes = self.oConfig["oTypes"].clone();
		self.oFilesToTypes = self.oConfig["oFilesToTypes"].clone();
		
		let oFilePath = self.oFilePath.clone(); // PathBuf clone?
		self.sFileName = String::from(oFilePath.file_name().unwrap().to_str().unwrap());
		
		self.oFileBlock = self.oFilesToTypes[&self.sFileName].clone();
	}

	// Загрузка регулярного выражения из конфига, ассоцированного с именем файла
	// Основные группы: date, message
	fn fnLoadRegExp(&mut self) -> Option<Regex> {
		let mut sRegExp = String::new();
		
		if !self.oFileBlock.is_null() {
			let sFileType = self.oFileBlock["sType"].as_str().unwrap();
			let oFileTypeRegExp: Value = self.oTypes[sFileType].clone();
	
			let sFileTypeRegExp = oFileTypeRegExp.as_str().unwrap();
			
			sRegExp = String::from(sFileTypeRegExp);
		}

		let mut oRegExp: Option<Regex> = None;
		
		if !sRegExp.is_empty() {
			oRegExp = Some(Regex::new(&sRegExp).unwrap());
		}

		return oRegExp;
	}

	// Загрузка формата даты из конфига
	fn fnLoadDatetimeFormat(&mut self) -> Option<String> {
		let mut oResult = None;

		if !self.oFileBlock.is_null() {
			oResult = Some(String::from(self.oFileBlock["sDateFormat"].as_str().unwrap()));
		}

		return oResult;
	}

	fn fnCountFileLines(&mut self) -> u32 {
		let oFilePath = self.oFilePath.clone();
		let sFileName = oFilePath.as_path();

		let file = File::open(&sFileName).expect("Unable to open");
		let mut reader = io::BufReader::new(&file);
		let mut lines = reader.lines();

		return lines.count() as u32;
	}
	
	fn fnReadLines(&mut self) {
		let oFilePath = self.oFilePath.clone();
		let sFileName = oFilePath.as_path();
	
		let (mut oDateFrom, mut oDateTo) = self.fnParseDateFilter();

		self.fnLoadBlockFromConfig();

		let oRegExp = self.fnLoadRegExp();
		let oDatetimeFormat = self.fnLoadDatetimeFormat();
		let mut sDatetimeFormat = String::new();
		
		if !oDatetimeFormat.is_none() {
			sDatetimeFormat = oDatetimeFormat.unwrap();
		}

		let bRegExpIsNotNone = !oRegExp.clone().is_none();
		let bDateFilterIsNotEmpty = !self.sDateFilter.is_empty();
		let bFilterIsNotEmpty = !self.sFilter.is_empty();
		let bReturnParsedList = match self.eFormatType {
			FormatTypes::Json => true,
			_ => false
		};

		let mut oLocalRegExp: Regex = Regex::new("").unwrap();

		if  bRegExpIsNotNone && bDateFilterIsNotEmpty {
			oLocalRegExp = oRegExp.clone().unwrap().clone();
		}

		self.aParsedList.clear();
		self.aStringList.clear();

		// Если стоит флаг показывать последнюю страницу.
		if self.bShowLastPage {
			self.iLinesCount = self.fnCountFileLines();
			self.iPageIndex = ceil(self.iLinesCount as f64/self.iPageSize as f64, 0) as u32;
		} else {
			self.iLinesCount = 0;
		}

		let file = File::open(&sFileName).expect("Unable to open");
		let mut reader = io::BufReader::new(&file);
		let mut lines = reader.lines();

		// let mut aResult = Vec::new();

		// let bFilter
	
		if self.iPageSize>0 {
			let iBeginIndex = (self.iPageIndex-1)*self.iPageSize;
			let mut iEndIndex = (self.iPageIndex)*self.iPageSize;
	
			let mut iIndex = 0;
			let mut sLine = String::new();
	
			loop {
				let oLine = lines.next();
	
				if oLine.is_none() {
					// println!("oLine {:?}", oLine);
					break;
				}
	
				sLine = oLine.unwrap().unwrap();

				let mut oCapturesFromLine: Option<Captures> = None;

				if bRegExpIsNotNone && bReturnParsedList {
					let oCaptures = oLocalRegExp.captures(&sLine).unwrap();
					oCapturesFromLine = Some(oCaptures);
				}

				// Если включен фильтр по дате, то парсим дату из sLine
				// if bRegExpIsNotNone && bDateFilterIsNotEmpty {
				// 	let oParsedDate = oCapturesFromLine.unwrap().name("date").clone();

				// 	// Проверяем есть ли распарсенная дата
				// 	if !oParsedDate.is_none() {
				// 		// Если дата есть, то парсим саму дату с использованием формата из конфига
				// 		let sParsedDate = oParsedDate.unwrap();
				// 		let mut oDateFromResult = NaiveDateTime::parse_from_str(sParsedDate.as_str(), sDatetimeFormat.as_str());
				// 	}
				// }
	
				if bFilterIsNotEmpty && !sLine.contains(&self.sFilter) {
					// println!("bFilterIsNotEmpty {:?}", bFilterIsNotEmpty);
					continue;
				}
	
				if iIndex >= iBeginIndex && iIndex < iEndIndex {
					// Если json формат и нужно вернуть распарсенные строки
					// println!("bReturnParsedList {:?}", bReturnParsedList);

					if bReturnParsedList {
						let oCaptureFromLine = oCapturesFromLine.unwrap();
						println!("oCaptureFromLine {:?}", oCaptureFromLine);

						let oHashMap: StringHashMap = oLocalRegExp
							.capture_names()
							.flatten()
							.filter_map(|n| Some((String::from(n), String::from(oCaptureFromLine.name(n).unwrap().as_str()))))
							.collect();
						
						self.aParsedList.push(oHashMap);

						// println!("self.aParsedList {:?}", self.aParsedList);
					} else {
						self.aStringList.push(String::from(sLine.trim()));

						// println!("self.aStringList {:?}", self.aStringList);
					}
				}
	
				if !self.bReturnCount && iIndex>=iEndIndex {
					break;
				}
	
				iIndex += 1;
			}

			self.iLinesCount = iIndex;
		}
	}
	
	fn fnLoadConfig(&mut self) {
		let mut oConfig: Value = Value::Null;

		if !self.sConfigAsString.is_empty() {
			oConfig = serde_json::from_str(&self.sConfigAsString).unwrap();
		} else if self.oConfigFilePath.is_file() {
			let sConfigFilePath = self.oConfigFilePath.as_path();
			let sConfigBuffer = read_to_string(sConfigFilePath).unwrap();

			println!("{:?}", sConfigBuffer);
	
			oConfig = serde_json::from_str(&sConfigBuffer).unwrap();
		} else {
			oConfig = serde_json::from_str(&"{}").unwrap();
		}
	
		self.oConfig = oConfig;
	}
}

fn main() {
	let oArgs = Args::from_args();

/*
	oConfig: Value,
	sConfigAsString: String,
	oConfigFilePath: PathBuf,
	oFilePath: &PathBuf, 
	sRegExp: String, 
	sDateFilter: String, 
	sFilter: String, 
	iPageIndex: u32, 
	iPageSize: u32, 
	bReturnLines: bool, 
	bReturnCount: bool,
	iLinesCount: u32,
	aParsedList: Vec<String>,
	aStringList: Vec<String>
*/

	let mut oLogParser = LogParser {
		oConfig: json!(""),
		sConfigAsString: String::from(""),
		oConfigFilePath: oArgs.oConfigFilePath.clone(),
		oFilePath: oArgs.oParseFilePath.clone(), 
		sRegExp: String::from(""), 
		sDateFilter: oArgs.sDateFilter.clone(), 
		sFilter: oArgs.sFilter.clone(), 
		iPageIndex: oArgs.iPageIndex, 
		bShowLastPage: oArgs.bShowLastPage,
		iPageSize: oArgs.iPageSize, 
		bReturnLines: false, 
		bReturnCount: false,
		iLinesCount: 0,
		aParsedList: vec![],
		aStringList: vec![],

		eFormatType: oArgs.eFormatType,

		oTypes: json!(""),
		oFilesToTypes: json!(""),	
		sFileName: String::from(""),
		oFileBlock: json!("")
	};

	let sOutput = oLogParser.fnParse();

	println!("{}", sOutput);
}
