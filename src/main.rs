extern crate tiny_http;

use tiny_http::{Server, Response, Header, HeaderField};
use azure_devops_rust_api::{Credential, git, pipelines};
use std::{error::Error, future::IntoFuture, str::FromStr};
use ascii::AsciiString;
use wildflower::Pattern;
use std::env;

struct Url(String);
struct Description(String);
struct Link(Url, Description);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "0.0.0.0:59991";
    
    let server = Server::http(addr).unwrap();

    let azdo_pat = env::var("SEARCH_SERVER_AZDO_PAT").unwrap();
    let azdo_org = env::var("SEARCH_SERVER_AZDO_ORG").unwrap();
    let azdo_proj = env::var("SEARCH_SERVER_AZDO_PROJ").unwrap();
    println!("Using PAT {}***", &azdo_pat[0..8]);

    let azdo_creds = Credential::from_pat(azdo_pat);
    let git_client = git::ClientBuilder::new(azdo_creds.clone()).build();
    let pipeline_client = pipelines::ClientBuilder::new(azdo_creds.clone()).build();

    println!("Listening at http://{}", addr);

    for request in server.incoming_requests() {
        let mut path_iter = request.url().splitn(2, '?');

        let links =
            if let Some(provider_name) = path_iter.next() {
                match provider_name {
                    "/repos" => {
                        let pattern = Pattern::new(path_iter.next().map_or("*".to_string(), |q| format!("*{}*", q.to_lowercase().replace('+', "*"))));

                        git_client
                            .repositories_client()
                            .list(&azdo_org, &azdo_proj)
                            .into_future().await
                            .map(|repos|
                                repos.value.into_iter()
                                    .filter(|r| pattern.matches(&r.name.to_lowercase()))
                                    .flat_map(|r| {
                                        r.web_url
                                            .map(|url| Link(Url(url), Description(format!("[{}] {}", azdo_proj, r.name))))
                                    })
                                    .collect())?
                    },
                    "/pipelines" => {
                        let pattern = Pattern::new(path_iter.next().map_or("*".to_string(), |q| format!("*{}*", q.to_lowercase().replace('+', "*"))));

                        pipeline_client
                            .pipelines_client()
                            .list(&azdo_org, &azdo_proj)
                            .into_future().await
                            .map(|pipelines| 
                                pipelines.value.into_iter()
                                    .filter(|p| pattern.matches(&p.name.to_lowercase()))
                                    .flat_map(|p| {
                                        println!("{:?}", p.links);

                                        if let Some(url) = p.links.web.map(|j| j.href) {
                                            Some(Link(Url(url.to_string()), Description(format!("[{}] {}\\{}", azdo_proj, p.folder, p.name).to_string())))
                                        }
                                        else {
                                            None
                                        }
                                    })
                                    .collect())?
                    },
                    _ => vec!()
                }
            }
            else { vec!() };

        let response = match links.len() {
            0 => {
                Response::from_string("")
                    .with_status_code(404)
            }
            1 => {
                Response::from_string("")
                    .with_header(Header {
                        field: HeaderField::from_str("Location").unwrap(),
                        value: AsciiString::from_str(&links.first().unwrap().0.0).unwrap()
                    })
                    .with_status_code(301)
            }
            _ => {
                let data = ["<!DOCTYPE html><html><style>li { font-size: 20px; margin: 5px; }</style><ul>".to_string()].into_iter()
                    .chain(links.into_iter().map(|link| format!("<li><a href=\"{}\">{}</a></li>", link.0.0, link.1.0)))
                    .chain(["</ul></html>".to_string()])
                    .fold(None, |ac, l| Some(ac.unwrap_or("".to_string()) + &l))
                    .unwrap();

                Response::from_string(data)
                    .with_header(Header {
                        field: HeaderField::from_str("Content-Type").unwrap(),
                        value: AsciiString::from_str("text/html; charset=UTF-8").unwrap()
                    })
            }
        };

        request.respond(response).unwrap();
    }

    println!("Fin");

    Ok(())
}

