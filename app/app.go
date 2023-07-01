package app

import (
	"container/heap"
	"encoding/json"
	"io"
	"net/http"

	"github.com/google/uuid"
	"github.com/sharithg/simcue/internal/file"
	"github.com/sharithg/simcue/internal/queue"
)

type MessageQueue struct {
	pq *queue.PriorityQueue
}

func NewMessageQueue() *MessageQueue {
	pq := &queue.PriorityQueue{}
	heap.Init(pq)
	return &MessageQueue{pq: pq}
}

func (mq *MessageQueue) SendMessage(w http.ResponseWriter, r *http.Request) {

	if r.Method != http.MethodPost {
		http.Error(w, "Invalid request method", http.StatusMethodNotAllowed)
		return
	}

	body, err := ReadPushMessageBody(r)

	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	id := uuid.New().String()

	m := &MessageResponse{MessageId: id}
	ms, err := json.Marshal(m)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	if file.WriteMessageToFile(body.Data, id) != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	item := &queue.Item{
		Value:    id,
		Priority: body.Priority,
	}

	heap.Push(mq.pq, item)

	w.WriteHeader(http.StatusOK)
	io.WriteString(w, string(ms))

}

func (mq *MessageQueue) GetMessages(w http.ResponseWriter, r *http.Request) {
	nItems := mq.pq.Len()

	if nItems == 0 {
		w.WriteHeader(http.StatusNoContent)
		return
	}

	item := heap.Pop(mq.pq).(*queue.Item)

	content, err := file.ReadMessageFromFile(item.Value)

	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	ms, err := PullResponseJson(item.Value, content)

	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	io.WriteString(w, string(ms))
}
