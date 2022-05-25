use crate::jsonrpc::errors::RpcError;
use crate::jsonrpc::json::{APIResponse, InvoiceReq, PegInReq, PegOutReq, Request, Response};
use crate::mint::SpendableCoin;
use minimint::modules::mint::tiered::coins::Coins;
use minimint_api::Amount;
use serde::Deserialize;

struct JsonRpc {
    client: reqwest::Client,
    host: String,
}
impl JsonRpc {
    pub fn new(host: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            host,
        }
    }

    async fn call(&self, request_object: Request) -> Result<APIResponse, Option<RpcError>> {
        let response = self
            .client
            .post(self.host.as_str())
            .json(&request_object)
            .send()
            .await
            .map_err(|_| None)?;

        //note: not using response.json since it would consume response and it would demand different approach bellow
        let response = response.bytes().await.map_err(|_| None)?;
        if let Ok(serde_json::Value::Null) = serde_json::from_slice(&response) {
            //notification 'null' response
            Ok(APIResponse::Empty)
        } else {
            //non-notification with ...
            match serde_json::from_slice::<Response>(&response) {
                //... a result
                Ok(Response {
                    result: Some(result),
                    ..
                }) => Ok(APIResponse::deserialize(result).expect("can't fail")),
                //.. an error
                Ok(Response {
                    error: Some(error), ..
                }) => Err(Some(error)),
                //either not a valid Response object OR a valid response with neither a result or error
                _ => panic!("this should not be reached"),
            }
        }
    }
    #[allow(dead_code)]
    pub async fn get_info<T: serde::Serialize>(
        &self,
        id: std::option::Option<T>, //<- IDK why I have to specify this
    ) -> Result<APIResponse, Option<RpcError>> {
        self.call(Request::standard("info", id)).await
    }
    #[allow(dead_code)]
    pub async fn get_pending<T: serde::Serialize>(
        &self,
        id: std::option::Option<T>,
    ) -> Result<APIResponse, Option<RpcError>> {
        self.call(Request::standard("pending", id)).await
    }
    #[allow(dead_code)]
    pub async fn get_events<T: serde::Serialize>(
        &self,
        params: u64,
        id: std::option::Option<T>,
    ) -> Result<APIResponse, Option<RpcError>> {
        self.call(Request::standard_with_params("events", params, id))
            .await
    }
    #[allow(dead_code)]
    pub async fn get_new_pegin_address<T: serde::Serialize>(
        &self,
        id: std::option::Option<T>,
    ) -> Result<APIResponse, Option<RpcError>> {
        self.call(Request::standard("pegin_address", id)).await
    }
    #[allow(dead_code)]
    pub async fn peg_in<T: serde::Serialize>(
        &self,
        params: PegInReq,
        id: std::option::Option<T>,
    ) -> Result<APIResponse, Option<RpcError>> {
        self.call(Request::standard_with_params("pegin", params, id))
            .await
    }
    #[allow(dead_code)]
    pub async fn peg_out<T: serde::Serialize>(
        &self,
        params: PegOutReq,
        id: std::option::Option<T>,
    ) -> Result<APIResponse, Option<RpcError>> {
        self.call(Request::standard_with_params("pegout", params, id))
            .await
    }
    #[allow(dead_code)]
    pub async fn spend<T: serde::Serialize>(
        &self,
        params: Amount,
        id: std::option::Option<T>,
    ) -> Result<APIResponse, Option<RpcError>> {
        self.call(Request::standard_with_params("spend", params.milli_sat, id))
            .await
    }
    #[allow(dead_code)]
    pub async fn lnpay<T: serde::Serialize>(
        &self,
        params: InvoiceReq,
        id: std::option::Option<T>,
    ) -> Result<APIResponse, Option<RpcError>> {
        self.call(Request::standard_with_params("lnpay", params, id))
            .await
    }
    #[allow(dead_code)]
    pub async fn reissue<T: serde::Serialize>(
        &self,
        params: Coins<SpendableCoin>,
        id: std::option::Option<T>,
    ) -> Result<APIResponse, Option<RpcError>> {
        self.call(Request::standard_with_params("reissue", params, id))
            .await
    }
}
impl Default for JsonRpc {
    fn default() -> Self {
        Self::new(String::from("http://127.0.0.1:8081/rpc"))
    }
}

#[cfg(test)]
mod tests {
    use crate::jsonrpc::json::InvoiceReq;

    #[tokio::test]
    async fn serial() {
        //let rpc = Client::default();
        let bolt11 = "lnbcrt10m1p3g0wkfpp50gx8zyvhhk0s5spd2r63adlx7naxyf90epxyl6v6dft4dmnuq5rsdq8w3jhxaqcqp2sp5e9rsfjtzauerup7gqjzn4j4frqq4wvpr5822mv708q32jt84lyjq9qyysgqjx9tp29s9qkux69tqkezhyykj43xe2c5jswj3dxq546hk6cedkjs5zntn2mqu3rnxrvma6wperz5eh3pne96w5u9khxzs2636txudwgqnyp8s9";
        let invoice_request: InvoiceReq = InvoiceReq {
            bolt11: bolt11.parse::<lightning_invoice::Invoice>().unwrap(),
        };
        //Serialize InvReq
        let ir_serial = dbg!(serde_json::to_string(&invoice_request).unwrap());
        //Deserialize ir_serial in a new InvReq
        let back_inv_req: InvoiceReq = serde_json::from_str(ir_serial.as_str()).unwrap();
        assert_eq!(invoice_request.bolt11, back_inv_req.bolt11);
    }
}
