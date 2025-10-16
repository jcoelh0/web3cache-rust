// testing purposes
import { io, Socket } from "socket.io-client";

const token: string =
  "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJhZGRyZXNzIjoiMHgwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwIn0.BpvndHO0pT3MgqSQs3nY905rPsk3poKcQHjU5nX2h4iqDWIWaulBYrFTrjxCLDrtRmccyr2-iRrp3Y9Z1ZesC-Ll4bxkIpiv8n_hmAt3qJohWaVcXYrBzyGMA-fIgjisNdfN3yhOlPWryb6H3XbuKiCxZntNub_ULZl9Gn5gEC0L1SurJ5lJKJBBN1VSy_-0yFYfcJCQohLm4eIpP-0tNQEy2hq3LwhzaXCAiXAmHKOvrw32M8Z-VPO_LxVROoEkzPKE1ZWU0tR1vTojsAIc2xLkiFBF1CeSbEliwr7M50PMsI_8Q8DY8-cRsinJDXX9dtecOVE2xuQoTqsjc5WoyA";

class Client {
  private socket: Socket;

  constructor() {
    this.socket = io("wss://web3cache.orangecomet.com", {
      path: "/socket.io", 
      withCredentials: true,
      extraHeaders: {
        cookie: `x-auth-token=${token}`,
      },
    });
    this.socket.on("connect", () => {
      console.log("connection!!");
      this.socket.emit("login");
      console.log("after login");
    });
    this.socket.on("transaction", (transaction: any) => {
      console.log("transaction", transaction);
    });
  }
}

const client = new Client();
