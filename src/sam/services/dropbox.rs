use dropbox_sdk::{files, UserAuthClient};
use dropbox_sdk::default_client::UserAuthDefaultClient;
use dropbox_sdk::default_client::NoauthDefaultClient;
use std::collections::VecDeque;
use std::thread;
use rouille::post_input;
use rouille::Request;
use rouille::Response;
use serde::{Serialize, Deserialize};

use std::io::prelude::*;

pub fn get_db_obj() -> Result<crate::sam::memory::Service, crate::sam::services::Error>{
    let mut pg_query = crate::sam::memory::PostgresQueries::default();
    pg_query.queries.push(crate::sam::memory::PGCol::String("dropbox".to_string()));
    pg_query.query_columns.push("identifier =".to_string());
    let service = crate::sam::memory::Service::select(None, None, None, Some(pg_query))?;
    Ok(service[0].clone())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DropboxAuth {
    pub url: String, // unique
    pub pkce: String,
}


pub fn get_auth_url() -> DropboxAuth {
    let pkce = dropbox_sdk::oauth2::PkceCode::new();
    let client_id = "ogyeqdms81svfke".to_string();
    let oauth2_flow = dropbox_sdk::oauth2::Oauth2Type::PKCE(pkce.clone());
    let url = dropbox_sdk::oauth2::AuthorizeUrlBuilder::new(&client_id, &oauth2_flow).build();
    DropboxAuth{
        url: url.to_string(),
        pkce: pkce.code.to_string(),
    }
}


pub fn finish_auth(pkce: String, auth_code: String) -> dropbox_sdk::oauth2::Authorization {

    let pkcee = dropbox_sdk::oauth2::PkceCode{code: pkce};

    let client_id = "ogyeqdms81svfke".to_string();
    let oauth2_flow = dropbox_sdk::oauth2::Oauth2Type::PKCE(pkcee);

    let auth = dropbox_sdk::oauth2::Authorization::from_auth_code(
        client_id,
        oauth2_flow,
        auth_code.trim().to_owned(),
        None,
    );



    auth
}

pub fn update_key(key: String, refresh: Option<String>){
    let mut service = crate::sam::memory::Service::new();
    service.identifier = "dropbox".to_string();
    match refresh{
        Some(refr) => {
            if refr.len() > 2 {
                service.key = refr;
            } else {
                let existing = get_db_obj().unwrap();
                service.key = existing.key;
            }
        },
        None => {
            let existing = get_db_obj().unwrap();
            service.key = existing.key;
        }
    }
    service.secret = key;
    service.endpoint = String::new();
    service.save().unwrap();
}

//  dropbox_sdk::files::delete_v2(&client, &dropbox_sdk::files::DeleteArg::new(path.clone()));



pub fn handle(_current_session: crate::sam::memory::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    if request.url() == "/api/services/dropbox" {

        let path_param = request.get_param("path");

        match path_param {
            Some(path) => {
                let files = get_paths(&path);
                return Ok(Response::json(&files));
            },
            None => {
                let files = get_paths("/");
                return Ok(Response::json(&files));
            }
        }
       
    }

    if request.url() == "/api/services/dropbox/download" {

        let path_param = request.get_param("path").unwrap();
        let data = download_file(&path_param).unwrap();
        
        let response = Response::from_data("", data);


        return Ok(response);
    }

    if request.url() == "/api/services/dropbox/auth/1" {
        let auth = get_auth_url();
        return Ok(Response::json(&auth));
    }

    if request.url() == "/api/services/dropbox/auth/2" {
        let input = post_input!(request, {
            pkce: String,
            auth_code: String
        })?;


        let mut auth = finish_auth(input.pkce, input.auth_code);
     

        let noc = NoauthDefaultClient::default();
        let new = auth.obtain_access_token(noc).unwrap();
        update_key(auth.save().unwrap(), Some(new.refresh_token));

        let response = Response::redirect_302("/services.html");
        return Ok(response);
    }




    Ok(Response::empty_404())
}



pub fn destroy_empty_directories(){
    let dropbox_destroy_empty_directories = thread::Builder::new().name("dropbox_destroy_empty_directories".to_string()).spawn(move || {
        let empties = crate::sam::services::dropbox::empty_directories();
        for e in empties{
            if is_path_empty(&e.clone()){
                delete(&e.clone());
            }
        }
    });

    match dropbox_destroy_empty_directories{
        Ok(_) => {
            log::info!("dropbox_destroy_empty_directories task started successfully");
        },
        Err(e) => {
            log::error!("failed to initialize dropbox_destroy_empty_directories task: {}", e);
        }
    }
}

pub fn create_sam_folder(){
    create_folder("/Sam");
}

pub fn create_folder(path: &str){
    let obj = get_db_obj().unwrap();
    let auth = dropbox_sdk::oauth2::Authorization::load("ogyeqdms81svfke".to_string(), &obj.secret).unwrap();
    let client = UserAuthDefaultClient::new(auth.clone());
    let _ = dropbox_sdk::files::create_folder_v2(&client, &dropbox_sdk::files::CreateFolderArg::new(path.to_string()));

}

// TODO: Cache Files
pub fn download_file(dropbox_path: &str) -> Result<Vec<u8>, String> {
    let obj = get_db_obj().unwrap();
    let auth = dropbox_sdk::oauth2::Authorization::load("ogyeqdms81svfke".to_string(), &obj.secret).unwrap();
    let client = UserAuthDefaultClient::new(auth.clone());
    let dropbox_file = dropbox_sdk::files::download(&client, &dropbox_sdk::files::DownloadArg::new(dropbox_path.to_string()), None, None);
    
    let mut body = dropbox_file.unwrap().unwrap().body.unwrap();

    let mut data = Vec::new();
    body.read_to_end(&mut data).expect("Unable to read data");

    Ok(data)
    

    // log::info!("dropbox_file: {:?}", );
}

pub fn delete(path: &str){
    let obj = get_db_obj().unwrap();
    let auth = dropbox_sdk::oauth2::Authorization::load("ogyeqdms81svfke".to_string(), &obj.secret).unwrap();
    let client = UserAuthDefaultClient::new(auth.clone());
    let _ = dropbox_sdk::files::delete_v2(&client, &dropbox_sdk::files::DeleteArg::new(path.to_string()));
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DropboxObject {
    pub path: String, // unique
    pub object_type: String,
}


pub fn get_paths(path: &str) -> Vec<DropboxObject>{
    let obj = get_db_obj().unwrap();
    let auth = dropbox_sdk::oauth2::Authorization::from_refresh_token("ogyeqdms81svfke".to_string(), obj.key);
    let client = UserAuthDefaultClient::new(auth.clone());

    let mut paths: Vec<DropboxObject> = Vec::new();

    match list_directory(&client, path, false) {
        Ok(Ok(iterator)) => {
            for entry_result in iterator {
                match entry_result {
                    Ok(Ok(files::Metadata::Folder(_entry))) => {
                        let path = _entry.path_display.unwrap_or(_entry.name);
                        let obj = DropboxObject {
                            path,
                            object_type: "folder".to_string(),
                        };
                        paths.push(obj);
                    },
                    Ok(Ok(files::Metadata::File(_entry))) => {
                        let path = _entry.path_display.unwrap_or(_entry.name);
                        let obj = DropboxObject {
                            path,
                            object_type: "file".to_string(),
                        };
                        paths.push(obj);
                    },
                    Ok(Ok(files::Metadata::Deleted(_entry))) => {
                        // panic!("unexpected deleted entry: {:?}", entry);
                    },
                    Ok(Err(_e)) => {
                        // eprintln!("Error from files/list_folder_continue: {}", _e);
                        break;
                    },
                    Err(_e) => {
                        // eprintln!("API request error: {}", _e);
                        break;
                    },
                }
            }
        },
        Ok(Err(_e)) => {
            eprintln!("Error from files/list_folder");
        },
        Err(_e) => {
            eprintln!("API request error");
        }
    }

    paths
}


pub fn empty_directories() -> Vec<String>{
    let obj = get_db_obj().unwrap();
    let auth = dropbox_sdk::oauth2::Authorization::load("ogyeqdms81svfke".to_string(), &obj.secret).unwrap();
    let client = UserAuthDefaultClient::new(auth.clone());


    let mut empty_directories: Vec<String> = Vec::new();

    match list_directory(&client, "/", true) {
        Ok(Ok(iterator)) => {
            for entry_result in iterator {
                match entry_result {
                    Ok(Ok(files::Metadata::Folder(_entry))) => {
                        let path = _entry.path_display.unwrap_or(_entry.name);

                        if is_path_empty(&path.clone()){
                            empty_directories.push(path.clone());
                        }
                    },
                    Ok(Ok(files::Metadata::File(_entry))) => {
                        // println!("File: {}", entry.path_display.unwrap_or(entry.name));
                    },
                    Ok(Ok(files::Metadata::Deleted(_entry))) => {
                        // panic!("unexpected deleted entry: {:?}", entry);
                    },
                    Ok(Err(_e)) => {
                        // eprintln!("Error from files/list_folder_continue: {}", _e);
                        break;
                    },
                    Err(_e) => {
                        // eprintln!("API request error: {}", _e);
                        break;
                    },
                }
            }
        },
        Ok(Err(_e)) => {
            eprintln!("Error from files/list_folder");
        },
        Err(_e) => {
            eprintln!("API request error");
        }
    }

    empty_directories

}


pub fn is_path_empty(path: &str) -> bool{

    log::info!("deleting dropbox path: {}", path);
    
    let obj = get_db_obj().unwrap();
    let auth = dropbox_sdk::oauth2::Authorization::load("ogyeqdms81svfke".to_string(), &obj.secret).unwrap();
    let client = UserAuthDefaultClient::new(auth.clone());
    

    let mut empty = true;
    match list_directory(&client, path, true) {
        Ok(Ok(iterator)) => {
            for entry_result in iterator {
                match entry_result {
                    Ok(Ok(files::Metadata::Folder(_entry))) => {
                        // empty = false;
                    },
                    Ok(Ok(files::Metadata::File(_entry))) => {
                        empty = false;
                        return empty;
                    },
                    Ok(Ok(files::Metadata::Deleted(_entry))) => {
                        // panic!("unexpected deleted entry: {:?}", entry);
                    },
                    Ok(Err(_e)) => {
                        // eprintln!("Error from files/list_folder_continue: {}", _e);
                        // break;
                    },
                    Err(_e) => {
                        // eprintln!("API request error: {}", _e);
                        // break;
                    },
                }
            }
        },
        Ok(Err(_e)) => {
            // eprintln!("Error from files/list_folder: {}", _e);
        },
        Err(_e) => {
            // eprintln!("API request error: {}", _e);
        }
    }

    empty

}





pub fn get_auth_from_env_or_prompt() -> dropbox_sdk::oauth2::Authorization {



    let client_id = String::new();

    let oauth2_flow = dropbox_sdk::oauth2::Oauth2Type::PKCE(dropbox_sdk::oauth2::PkceCode::new());
    let url = dropbox_sdk::oauth2::AuthorizeUrlBuilder::new(&client_id, &oauth2_flow)
        .build();
    eprintln!("Open this URL in your browser:");
    eprintln!("{}", url);
    eprintln!();
    let auth_code = String::new();

    dropbox_sdk::oauth2::Authorization::from_auth_code(
        client_id,
        oauth2_flow,
        auth_code.trim().to_owned(),
        None,
    )
}


fn list_directory<'a, T: UserAuthClient>(client: &'a T, path: &str, recursive: bool)
    -> dropbox_sdk::Result<Result<DirectoryIterator<'a, T>, files::ListFolderError>>
{
    assert!(path.starts_with('/'), "path needs to be absolute (start with a '/')");
    let requested_path = if path == "/" {
        // Root folder should be requested as empty string
        String::new()
    } else {
        path.to_owned()
    };
    match files::list_folder(
        client,
        &files::ListFolderArg::new(requested_path)
            .with_recursive(recursive))
    {
        Ok(Ok(result)) => {
            let cursor = if result.has_more {
                Some(result.cursor)
            } else {
                None
            };

            Ok(Ok(DirectoryIterator {
                client,
                cursor,
                buffer: result.entries.into(),
            }))
        },
        Ok(Err(e)) => Ok(Err(e)),
        Err(e) => Err(e),
    }
}

struct DirectoryIterator<'a, T: UserAuthClient> {
    client: &'a T,
    buffer: VecDeque<files::Metadata>,
    cursor: Option<String>,
}

impl<'a, T: UserAuthClient> Iterator for DirectoryIterator<'a, T> {
    type Item = dropbox_sdk::Result<Result<files::Metadata, files::ListFolderContinueError>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(entry) = self.buffer.pop_front() {
            Some(Ok(Ok(entry)))
        } else if let Some(cursor) = self.cursor.take() {
            match files::list_folder_continue(self.client, &files::ListFolderContinueArg::new(cursor)) {
                Ok(Ok(result)) => {
                    self.buffer.extend(result.entries);
                    if result.has_more {
                        self.cursor = Some(result.cursor);
                    }
                    self.buffer.pop_front().map(|entry| Ok(Ok(entry)))
                },
                Ok(Err(e)) => Some(Ok(Err(e))),
                Err(e) => Some(Err(e)),
            }
        } else {
            None
        }
    }
}