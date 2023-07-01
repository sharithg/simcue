package main

import (
	"log"
	"net/http"

	"github.com/sharithg/simcue/app"
)

const ServerPort = ":3333"

func main() {
	// Create a priority queue, put the items in it, and
	// establish the priority queue (heap) invariants.
	mq := app.NewMessageQueue()

	mux := http.NewServeMux()

	mux.Handle("/push", app.Middleware(http.HandlerFunc(mq.SendMessage)))
	mux.Handle("/pull", app.Middleware(http.HandlerFunc(mq.GetMessages)))

	log.Fatal(http.ListenAndServe(ServerPort, mux))
}
