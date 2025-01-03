package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net"
	"os"

	"github.com/getlantern/systray"
)

type Metrics struct {
	Keypresses      int     `json:"keypresses"`
	MouseClicks     int     `json:"mouse_clicks"`
	MouseDistanceIn float64 `json:"mouse_distance_in"`
	MouseDistanceMi float64 `json:"mouse_distance_mi"`
	ScrollSteps     int     `json:"scroll_steps"`
}

var (
	mKeyPresses    *systray.MenuItem
	mMouseClicks   *systray.MenuItem
	mMouseDistance *systray.MenuItem
	mScrollSteps   *systray.MenuItem
	listener       net.Listener
)

const sockAddr = "/tmp/kawaiilogger.sock"
var isMenuInitialized = false

func main() {
    log.Println("Starting Go application...")

    if err := os.Remove(sockAddr); err != nil && !os.IsNotExist(err) {
        log.Fatalf("Failed to remove existing socket file: %v", err)
    }

    log.Println("Starting systray...")
    go startSocketListener() 
    systray.Run(onReady, onExit) 
}

func startSocketListener() {
    log.Println("Creating Unix socket...")
    var err error
    listener, err = net.Listen("unix", sockAddr)
    if err != nil {
        log.Fatalf("Failed to create Unix socket: %v", err)
    }
    defer listener.Close()
    log.Printf("Unix socket created at %s\n", sockAddr)

    for {
        conn, err := listener.Accept()
        if err != nil {
            log.Printf("Error accepting connection: %v", err)
            continue
        }
        log.Println("Client connected")
        go handleConnection(conn)
    }
}

func connectToSocket() {
	log.Println("Attempting to connect to socket...")
	conn, err := net.Dial("unix", sockAddr)
	if err != nil {
		log.Printf("Failed to connect to socket: %v\n", err)
		return
	}
	defer conn.Close()
	log.Println("Successfully connected to socket")

	handleConnection(conn)
}

func handleConnection(conn net.Conn) {
	buffer := make([]byte, 1024)
	for {
		n, err := conn.Read(buffer)
		if err != nil {
			log.Printf("Error reading from socket: %v\n", err)
			return
		}

		var metrics Metrics
		if err := json.Unmarshal(buffer[:n], &metrics); err != nil {
			log.Printf("Error unmarshaling metrics: %v\n", err)
			continue
		}

		log.Printf("Received metrics: %+v\n", metrics)
		updateMenuItems(&metrics)
	}
}

func onReady() {
	log.Println("systray.OnReady called")
	systray.SetTitle("📊")
	systray.SetTooltip("KawaiiLogger")

	mKeyPresses = systray.AddMenuItem("Keypresses: 0", "Number of keypresses")
	mMouseClicks = systray.AddMenuItem("Mouse Clicks: 0", "Number of mouse clicks")
	mMouseDistance = systray.AddMenuItem("Mouse Travel: 0 in / 0 mi", "Distance moved by mouse")
	mScrollSteps = systray.AddMenuItem("Scroll Steps: 0", "Number of scroll steps")

	systray.AddSeparator()
	mQuit := systray.AddMenuItem("Quit", "Quit the application")

	go func() {
		<-mQuit.ClickedCh
		log.Println("Quit clicked, cleaning up...")
		cleanup()
		systray.Quit()
	}()

	isMenuInitialized = true
	log.Println("systray.OnReady completed")
}


func onExit() {
	log.Println("systray.OnExit called")
	cleanup()
}

func cleanup() {
	log.Println("Cleaning up...")
	if listener != nil {
		listener.Close()
	}
	os.Remove(sockAddr)
}

func updateMenuItems(metrics *Metrics) {
	if !isMenuInitialized {
		log.Println("Menu items not initialized, skipping update")
		return
	}

	if mKeyPresses == nil || mMouseClicks == nil || mMouseDistance == nil || mScrollSteps == nil {
		log.Println("Menu items are nil, skipping update")
		return
	}

	mKeyPresses.SetTitle(fmt.Sprintf("Keypresses: %d", metrics.Keypresses))
	mMouseClicks.SetTitle(fmt.Sprintf("Mouse Clicks: %d", metrics.MouseClicks))
	mMouseDistance.SetTitle(fmt.Sprintf("Mouse Travel: %.2f in / %.2f mi",
		metrics.MouseDistanceIn, metrics.MouseDistanceMi))
	mScrollSteps.SetTitle(fmt.Sprintf("Scroll Steps: %d", metrics.ScrollSteps))
}
