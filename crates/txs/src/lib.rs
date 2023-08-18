pub struct TxsDataHandler<State> {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub json: String,
    pub state: State,
}
