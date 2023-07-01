package app

import (
	"net/http"
	"net/http/httptest"

	"github.com/sirupsen/logrus"
)

func Middleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		// Create a new ResponseWriter that captures the response
		wrappedWriter := httptest.NewRecorder()

		// Serve the request using the wrapped ResponseWriter
		next.ServeHTTP(wrappedWriter, r)

		// Copy the response status code
		w.WriteHeader(wrappedWriter.Code)

		// Copy the response headers from the wrapped ResponseWriter
		for key, values := range wrappedWriter.Header() {
			for _, value := range values {
				w.Header().Add(key, value)
			}
		}

		// Copy the response body from the wrapped ResponseWriter
		w.Write(wrappedWriter.Body.Bytes())

		// Logging
		logrus.WithFields(logrus.Fields{
			"Method":        r.Method,
			"RequestURI":    r.RequestURI,
			"StatusCode":    wrappedWriter.Code,
			"ContentLength": wrappedWriter.Body.Len(),
		}).Info("Request processed")
	})
}
