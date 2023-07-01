package app

import (
	"encoding/json"
	"io"
	"net/http"
)

type MessagePushRequest struct {
	// Define your request structure here
	Priority int    `json:"priority"`
	Data     string `json:"data"`
}

func ReadPushMessageBody(r *http.Request) (*MessagePushRequest, error) {
	// Read the request body
	body, err := io.ReadAll(r.Body)
	if err != nil {
		// Handle the error
		return nil, err
	}

	// Close the request body
	defer r.Body.Close()

	// Convert the request body to JSON
	var requestData MessagePushRequest
	err = json.Unmarshal(body, &requestData)
	if err != nil {
		return nil, err
	}

	return &requestData, nil
}

type MessageResponse struct {
	MessageId string `json:"messageId"`
}

type PullResponse struct {
	MessageId string `json:"messageId,omitempty"`
	Data      string `json:"data,omitempty"`
}

func PullResponseJson(id string, d string) ([]byte, error) {
	msg := &PullResponse{MessageId: id, Data: d}
	ms, err := json.Marshal(msg)
	if err != nil {
		return nil, err
	}

	return ms, nil
}
