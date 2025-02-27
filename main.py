package main

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"github.com/spf13/cobra"
)

var (
	clashConfigDir  string
	clashAPIBaseURL string
	clashAPIPort    string
)

func init() {
	homeDir, _ := os.UserHomeDir()
	clashConfigDir = filepath.Join(homeDir, ".config", "clash")
	clashAPIPort = "9090"
	clashAPIBaseURL = "http://127.0.0.1:" + clashAPIPort
}

func main() {
	var rootCmd = &cobra.Command{
		Use:   "clash-cli",
		Short: "A CLI tool to manage Clash proxy",
		Long:  `Clash-CLI is a command line tool to manage Clash proxy configurations, switch profiles, and modify settings.`,
	}

	// 列出所有配置文件
	var listCmd = &cobra.Command{
		Use:   "list",
		Short: "List all available configurations",
		Run: func(cmd *cobra.Command, args []string) {
			files, err := ioutil.ReadDir(clashConfigDir)
			if err != nil {
				fmt.Printf("Error reading config directory: %v\n", err)
				return
			}

			fmt.Println("Available configurations:")
			for _, file := range files {
				if !file.IsDir() && (strings.HasSuffix(file.Name(), ".yaml") || strings.HasSuffix(file.Name(), ".yml")) {
					fmt.Printf("  - %s\n", file.Name())
				}
			}
		},
	}

	// 切换配置文件
	var switchCmd = &cobra.Command{
		Use:   "switch [config_file]",
		Short: "Switch to a different configuration",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			configFile := args[0]
			if !strings.HasSuffix(configFile, ".yaml") && !strings.HasSuffix(configFile, ".yml") {
				configFile += ".yaml"
			}

			configPath := filepath.Join(clashConfigDir, configFile)
			if _, err := os.Stat(configPath); os.IsNotExist(err) {
				fmt.Printf("Config file %s does not exist\n", configFile)
				return
			}

			// 通过API切换配置
			url := clashAPIBaseURL + "/configs"
			payload := map[string]string{"path": configPath}
			jsonPayload, _ := json.Marshal(payload)

			req, _ := http.NewRequest("PUT", url, strings.NewReader(string(jsonPayload)))
			req.Header.Set("Content-Type", "application/json")

			client := &http.Client{}
			resp, err := client.Do(req)
			if err != nil {
				fmt.Printf("Error switching config: %v\n", err)
				fmt.Println("Make sure Clash is running and external controller is enabled")
				return
			}
			defer resp.Body.Close()

			if resp.StatusCode == 204 {
				fmt.Printf("Successfully switched to %s\n", configFile)
			} else {
				body, _ := ioutil.ReadAll(resp.Body)
				fmt.Printf("Failed to switch config: %s\n", string(body))
			}
		},
	}

	// 获取当前代理信息
	var statusCmd = &cobra.Command{
		Use:   "status",
		Short: "Get current Clash status",
		Run: func(cmd *cobra.Command, args []string) {
			// 获取代理信息
			resp, err := http.Get(clashAPIBaseURL + "/proxies")
			if err != nil {
				fmt.Printf("Error getting proxy status: %v\n", err)
				fmt.Println("Make sure Clash is running and external controller is enabled")
				return
			}
			defer resp.Body.Close()

			body, _ := ioutil.ReadAll(resp.Body)
			var proxyData map[string]interface{}
			json.Unmarshal(body, &proxyData)

			fmt.Println("Clash Status:")
			fmt.Println("============")

			// 获取配置信息
			configResp, err := http.Get(clashAPIBaseURL + "/configs")
			if err == nil {
				defer configResp.Body.Close()
				configBody, _ := ioutil.ReadAll(configResp.Body)
				var configData map[string]interface{}
				json.Unmarshal(configBody, &configData)

				if mode, ok := configData["mode"].(string); ok {
					fmt.Printf("Mode: %s\n", mode)
				}
				if port, ok := configData["port"].(float64); ok {
					fmt.Printf("HTTP Port: %d\n", int(port))
				}
				if port, ok := configData["socks-port"].(float64); ok {
					fmt.Printf("SOCKS Port: %d\n", int(port))
				}
			}

			// 显示选择的代理
			if proxies, ok := proxyData["proxies"].(map[string]interface{}); ok {
				fmt.Println("\nSelected Proxies:")
				showSelectedProxy(proxies, "GLOBAL")
				showSelectedProxy(proxies, "Proxy")
			}
		},
	}

	// 修改代理模式
	var setModeCmd = &cobra.Command{
		Use:   "mode [mode]",
		Short: "Set Clash mode (Global, Rule, Direct)",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			mode := strings.ToLower(args[0])
			valid := false
			for _, validMode := range []string{"global", "rule", "direct"} {
				if mode == validMode {
					valid = true
					break
				}
			}

			if !valid {
				fmt.Println("Invalid mode. Use one of: Global, Rule, Direct")
				return
			}

			url := clashAPIBaseURL + "/configs"
			payload := map[string]string{"mode": mode}
			jsonPayload, _ := json.Marshal(payload)

			req, _ := http.NewRequest("PATCH", url, strings.NewReader(string(jsonPayload)))
			req.Header.Set("Content-Type", "application/json")

			client := &http.Client{}
			resp, err := client.Do(req)
			if err != nil {
				fmt.Printf("Error setting mode: %v\n", err)
				return
			}
			defer resp.Body.Close()

			if resp.StatusCode == 204 {
				fmt.Printf("Successfully set mode to %s\n", mode)
			} else {
				body, _ := ioutil.ReadAll(resp.Body)
				fmt.Printf("Failed to set mode: %s\n", string(body))
			}
		},
	}

	// 选择代理节点
	var selectProxyCmd = &cobra.Command{
		Use:   "select [group] [proxy]",
		Short: "Select a proxy from a group",
		Args:  cobra.ExactArgs(2),
		Run: func(cmd *cobra.Command, args []string) {
			group := args[0]
			proxy := args[1]

			url := fmt.Sprintf("%s/proxies/%s", clashAPIBaseURL, group)
			payload := map[string]string{"name": proxy}
			jsonPayload, _ := json.Marshal(payload)

			req, _ := http.NewRequest("PUT", url, strings.NewReader(string(jsonPayload)))
			req.Header.Set("Content-Type", "application/json")

			client := &http.Client{}
			resp, err := client.Do(req)
			if err != nil {
				fmt.Printf("Error selecting proxy: %v\n", err)
				return
			}
			defer resp.Body.Close()

			if resp.StatusCode == 204 {
				fmt.Printf("Successfully selected %s in group %s\n", proxy, group)
			} else {
				body, _ := ioutil.ReadAll(resp.Body)
				fmt.Printf("Failed to select proxy: %s\n", string(body))
			}
		},
	}

	// 列出代理组和节点
	var listProxiesCmd = &cobra.Command{
		Use:   "proxies",
		Short: "List all proxy groups and proxies",
		Run: func(cmd *cobra.Command, args []string) {
			resp, err := http.Get(clashAPIBaseURL + "/proxies")
			if err != nil {
				fmt.Printf("Error getting proxies: %v\n", err)
				return
			}
			defer resp.Body.Close()

			body, _ := ioutil.ReadAll(resp.Body)
			var proxyData map[string]interface{}
			json.Unmarshal(body, &proxyData)

			if proxies, ok := proxyData["proxies"].(map[string]interface{}); ok {
				fmt.Println("Proxy Groups:")
				fmt.Println("============")

				for name, data := range proxies {
					if proxyInfo, ok := data.(map[string]interface{}); ok {
						if proxyType, ok := proxyInfo["type"].(string); ok && proxyType == "Selector" {
							fmt.Printf("\n[%s]\n", name)
							if all, ok := proxyInfo["all"].([]interface{}); ok {
								for i, proxy := range all {
									if proxyStr, ok := proxy.(string); ok {
										// 标记当前选中的节点
										now := " "
										if nowProxy, ok := proxyInfo["now"].(string); ok && nowProxy == proxyStr {
											now = "*"
										}
										fmt.Printf("  %s %d. %s\n", now, i+1, proxyStr)
									}
								}
							}
						}
					}
				}
			}
		},
	}

	// 测试延迟
	var testLatencyCmd = &cobra.Command{
		Use:   "test [proxy]",
		Short: "Test proxy latency",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			proxy := args[0]
			url := fmt.Sprintf("%s/proxies/%s/delay", clashAPIBaseURL, proxy)
			
			req, _ := http.NewRequest("GET", url+"?timeout=5000&url=http://www.gstatic.com/generate_204", nil)
			
			client := &http.Client{}
			resp, err := client.Do(req)
			if err != nil {
				fmt.Printf("Error testing latency: %v\n", err)
				return
			}
			defer resp.Body.Close()
			
			body, _ := ioutil.ReadAll(resp.Body)
			var latencyData map[string]interface{}
			json.Unmarshal(body, &latencyData)
			
			if delay, ok := latencyData["delay"].(float64); ok {
				fmt.Printf("Proxy: %s, Latency: %.0f ms\n", proxy, delay)
			} else {
				fmt.Printf("Failed to test latency: %s\n", string(body))
			}
		},
	}

	rootCmd.AddCommand(listCmd)
	rootCmd.AddCommand(switchCmd)
	rootCmd.AddCommand(statusCmd)
	rootCmd.AddCommand(setModeCmd)
	rootCmd.AddCommand(selectProxyCmd)
	rootCmd.AddCommand(listProxiesCmd)
	rootCmd.AddCommand(testLatencyCmd)

	rootCmd.Execute()
}

func showSelectedProxy(proxies map[string]interface{}, group string) {
	if proxyData, ok := proxies[group].(map[string]interface{}); ok {
		if now, ok := proxyData["now"].(string); ok {
			fmt.Printf("  %s: %s\n", group, now)
		}
	}
}
