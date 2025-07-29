import Alpine from "alpinejs";
import "./index";

export function initAlpine(connect: (game: string) => void, runSinglePlayer: (settings: any) => void) {
  Alpine.store("client", {
    async joinAndConnect(gameId: string) {
      const id = await (await fetch(window.backendUrl + "/join-game/" + gameId)).json();
      connect(id);
    },

    connect,

    runSinglePlayer
  })

  Alpine.data("games", () => ({
    games: [],

    init() {
      const eventSource = new EventSource(window.backendUrl + "/games");
      eventSource.onmessage = (event) => {
        this.message(JSON.parse(event.data));
      }
    },

    message(data: object) {
      this.games = Object.entries(data).map(([id, game]) => ({...game, id}));
    },
  }));
}
