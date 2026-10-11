const e = new Error("boom");
console.log(String(e), `${e}`, e.toString(), e.message);
try { throw new Error("inner"); } catch (err) { console.log(`got ${err}`); }
try { throw "plain"; } catch (err) { console.log(`got ${err}`); }
class NotFound extends Error {
  constructor(public key: string) {
    super(`no ${key}`);
    this.name = "NotFound";
  }
}
try { throw new NotFound("k"); } catch (err) { console.log(`${err}`, String(err)); }
