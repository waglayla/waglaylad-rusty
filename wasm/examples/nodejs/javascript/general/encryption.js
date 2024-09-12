const waglayla = require('../../../../nodejs/waglayla');

waglayla.initConsolePanicHook();

(async () => {

    let encrypted = waglayla.encryptXChaCha20Poly1305("my message", "my_password");
    console.log("encrypted:", encrypted);
    let decrypted = waglayla.decryptXChaCha20Poly1305(encrypted, "my_password");
    console.log("decrypted:", decrypted);

})();
