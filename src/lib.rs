

mod saori {
	use regex::Regex;
	
	pub enum RequestType{
		GetVersion,
		Execute
	}

	#[allow(unused)]
	pub enum ResponseType{
		Ok,
		BadRequest,
		InternalServerError
	}

	//さおりリクエスト
	#[allow(unused)]
	pub struct SaoriRequest {
		pub request_type: RequestType,
		pub args: Vec<String>,
		pub security_level: String,
		pub sender: String,
		pub charset: String
	}

	//さおりレスポンス
	pub struct SaoriResponse {
		pub response_type: ResponseType,
		pub values: Vec<String>,
		pub result: String
	}

	//リクエスト処理
	pub fn request(input:&str) -> String {
		//解析
		let request = parse_request(input);

		//処理よびだし
		if let Ok(request) = request{
			if let RequestType::GetVersion = request.request_type {
				//Get Versionは自動返答
				String::from("SAORI/1.0 200 OK\r\nCharset: Shift_JIS\r\n\r\n")
			}
			else {
				//SAORIメイン処理
				//String::from("SAORI/1.0 200 OK\r\nCharset: Shift_JIS\r\nResult: testtesttest\r\n\r\n")
				let response = implement::request(request);
				make_response(&response)
			}
		}
		else {
			//パース仕切れないデータ
			String::from("SAORI/1.0 400 Bad Request\r\nCharset: Shift_JIS\r\n\r\n")
		}
	}

	//出力作成
	pub fn make_response(input:&SaoriResponse) -> String{
		let status = match input.response_type {
			ResponseType::Ok => "200 OK",
			ResponseType::BadRequest => "400 Bad Request",
			ResponseType::InternalServerError => "500 Internal Server Error"
		};

		let mut response = format!("SAORI/1.0 {}\r\nCharset: Shift_JIS\r\nResult: {}\r\n", status, input.result);
		for index in 0..input.values.len(){
			let v = format!("Value{}: {}\r\n", index, input.values[index]);
			response.push_str(v.as_str());
		}
		response.push_str("\r\n");
		return response;
	}

	//入力パース
	pub fn parse_request(input:&str) -> Result<SaoriRequest, &str>{
		let argument_pattern = Regex::new(r"^Argument(?<index>[1-9]?[0-9]+): (?<body>.+)").unwrap();
		let security_level_pattern = Regex::new(r"^SecurityLevel: (?<level>.+)").unwrap();
		let charset_pattern = Regex::new(r"^Charset: (?<charset>.+)").unwrap();
		let sender_pattern = Regex::new(r"^Sender: (?<sender>.+)").unwrap();


		let mut lines = input.split("\r\n");

		//最初の行をチェック
		let first = lines.next();

		let mut args:Vec<String> = Vec::new();
		let mut sender = String::new();
		let mut security_level = String::new();
		let mut charset = String::new();

		if let Some(first) = first{
			if first.starts_with("GET Version SAORI"){				
				return Ok(SaoriRequest {
					request_type: RequestType::GetVersion, 
					args: Vec::new(),
					security_level,
					sender,
					charset
				});
			} else if first.starts_with("EXECUTE SAORI"){

				//引数収集
				for line in lines {

					if let Some(c) = argument_pattern.captures(line){
						//Argument*
						let index:usize = c["index"].parse().unwrap();
						let body:String = c["body"].to_string();

						if index >= args.len(){
							args.resize(index+1, String::new());
						}
						args[index] = body;
					}
					else if let Some(c) = charset_pattern.captures(line){
						charset = c["charset"].to_string();
					}
					else if let Some(c) = sender_pattern.captures(line){
						sender = c["sender"].to_string();
					}
					else if let Some(c) = security_level_pattern.captures(line){
						security_level = c["level"].to_string();
					}
				}

				return Ok(SaoriRequest {
					request_type: RequestType::Execute,
					args,
					security_level,
					sender,
					charset
				});
			}
		}
		
		return Err("プロトコルが変です");
	}

	//さおり実装
	mod implement{
    	use super::*;

		pub fn request(request:SaoriRequest) -> SaoriResponse {
			//とりあえず、最初の引数をresultとして返してみる
			if request.args.len() >= 1 {
				SaoriResponse{
					response_type: ResponseType::Ok,
					result: request.args[0].clone(),
					values: vec![String::from("test1"), String::from("test2")]
				}
			}
			else {
				SaoriResponse {
					response_type: ResponseType::BadRequest,
					result: String::new(),
					values: Vec::new()
				}
			}
		}
	}

}


//DLL公開関数
use windows::Win32;
use std::borrow::Borrow;
use encoding_rs;

#[no_mangle]
pub extern "C" fn load(h: Win32::Foundation::HGLOBAL, _: i32) -> i32{
	unsafe{
		let _ = Win32::Foundation::GlobalFree(h);
	}
	1
}

#[no_mangle]
pub  extern "C" fn unload() -> i32{
	1
}

#[no_mangle]
pub extern "C" fn request(h: Win32::Foundation::HGLOBAL, len: &mut i32) -> Win32::Foundation::HGLOBAL{
	let req:String;

	//読み出し
	unsafe {
		//Rust側のメモリにコピーをとる
		let size = *len as usize;
		let mut v :Vec<u8>=Vec::with_capacity(size);
		v.set_len(size);
		std::ptr::copy(h.0 as *const _, v.as_mut_ptr(), size);
		let _ = Win32::Foundation::GlobalFree(h);
		
		//Shift_JISで最初からパースしておく
		let sjis = encoding_rs::SHIFT_JIS.decode(v.borrow());
		req = sjis.0.to_string();
	}
	
	//SAORI実行
	let res = saori::request(req.as_str());

	//書き出し
	unsafe{
		//Shift_JISに戻す
		let e = encoding_rs::SHIFT_JIS.encode(res.as_str());

		//書き込んで返す
		*len = (e.0.len() + 1) as i32;
		let mem = Win32::System::Memory::GlobalAlloc(Win32::System::Memory::GMEM_FIXED, *len as usize).unwrap();
		std::ptr::copy(e.0.as_ptr() as *const u8, mem.0 as *mut u8, e.0.len());
		std::ptr::write_bytes(mem.0.offset(e.0.len() as isize), 0, 1);
		return mem;
	}
}


//てすと
#[test]
fn test(){
	//let r = saori::request("GET Version SAORI/1.0\r\nCharset: Shift_JIS\r\nSender: Test\r\n\r\n");
	let r = saori::request("EXECUTE SAORI/1.0\r\nCharset: Shift_JIS\r\nSender: Test\r\n\r\n");
	println!("{}", r);
}