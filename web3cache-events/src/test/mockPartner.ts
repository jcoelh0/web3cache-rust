import express from "express";
import axios from "axios";

const port = 3002;
const webhookURL =
  "http://localhost:3001/subscription-update/633a1cb39574b8421f66882e";
const app = express();
app.use(express.json());

class MockPartner {
  constructor() {
    app.post("/analize", (req: express.Request, res: express.Response) => {
      console.log(JSON.stringify(req.body));
      res.send("Ok");
    });
    app.listen(port, () => console.log("Partner is listening"));
  }
  public async postSubscribe() {
    var data = {
      url: "http://localhost:" + port + "/burn",
      conract_id: "renga_mainnet",
      block_number: "",
    };

    const headers = {
      "Content-Type": "application/json",
      "x-webhook-apikey":
        "qgfQ4c4dikVNAzYeBXqHhYQ8nOOmM9SxTVNfTQ2XBGtsou7bMnW0tof0MwupPS9AT2aLcCIM7bGVrFBLLECzUdoNsUzNHi5HLulqR94vwwHfDdB0FtEFSBlCwXAvZffV",
    };

    try {
      const res = await axios.post(webhookURL, data, { headers });
      console.log(`subscribe response ${res.data}`);
    } catch (error: any) {
      console.log(error.response.data);
    }
  }
}
const partner = new MockPartner();
partner.postSubscribe();
