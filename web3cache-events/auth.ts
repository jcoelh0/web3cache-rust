/* 
module.exports = async function socketAuth(handshake) {
    try {
      const value = handshake.headers.cookie
        .split("; ")
        .reduce((prev, current) => {
          const [name, ...value] = current.split("=");
          prev[name] = value.join("=");
          return prev;
        }, {});
      const token = value["x-auth-token"];
      debugauth("socketAuth", token);
  
      jwt.verify(token, config.get("jwtPrivateKey"));
      const validSessionToken = await Token.findOne({ token });
      return validSessionToken;
    } catch (ex) {
      debugauth("ERROR in auth: ", ex.message);
      return false;
    }
  };
   */