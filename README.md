# tugboat_rs
> Overengineered Cloudflare Challenge Discord bot.


## Framework
the dream
```rs
use framework::{
	InteractionRouter,
	Responder
};

fn ping() -> impl Responder {
	"Pong!"
}

fn main() {
	let router = InteractionRouter::new();
	router.command("ping", ping);
}
```
