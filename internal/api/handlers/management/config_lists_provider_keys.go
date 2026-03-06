package management

import (
	"encoding/json"
	"fmt"
	"strings"

	"github.com/gin-gonic/gin"
	"github.com/router-for-me/CLIProxyAPI/v6/internal/config"
)

// gemini-api-key: []GeminiKey
func (h *Handler) GetGeminiKeys(c *gin.Context) {
	c.JSON(200, gin.H{"gemini-api-key": h.cfg.GeminiKey})
}
func (h *Handler) PutGeminiKeys(c *gin.Context) {
	data, err := c.GetRawData()
	if err != nil {
		c.JSON(400, gin.H{"error": "failed to read body"})
		return
	}
	var arr []config.GeminiKey
	if err = json.Unmarshal(data, &arr); err != nil {
		var obj struct {
			Items []config.GeminiKey `json:"items"`
		}
		if err2 := json.Unmarshal(data, &obj); err2 != nil || len(obj.Items) == 0 {
			c.JSON(400, gin.H{"error": "invalid body"})
			return
		}
		arr = obj.Items
	}
	h.cfg.GeminiKey = append([]config.GeminiKey(nil), arr...)
	h.cfg.SanitizeGeminiKeys()
	h.persist(c)
}
func (h *Handler) PatchGeminiKey(c *gin.Context) {
	type geminiKeyPatch struct {
		APIKey         *string            `json:"api-key"`
		Prefix         *string            `json:"prefix"`
		BaseURL        *string            `json:"base-url"`
		ProxyURL       *string            `json:"proxy-url"`
		Headers        *map[string]string `json:"headers"`
		ExcludedModels *[]string          `json:"excluded-models"`
	}
	var body struct {
		Index *int            `json:"index"`
		Match *string         `json:"match"`
		Value *geminiKeyPatch `json:"value"`
	}
	if err := c.ShouldBindJSON(&body); err != nil || body.Value == nil {
		c.JSON(400, gin.H{"error": "invalid body"})
		return
	}
	targetIndex := -1
	if body.Index != nil && *body.Index >= 0 && *body.Index < len(h.cfg.GeminiKey) {
		targetIndex = *body.Index
	}
	if targetIndex == -1 && body.Match != nil {
		match := strings.TrimSpace(*body.Match)
		if match != "" {
			for i := range h.cfg.GeminiKey {
				if h.cfg.GeminiKey[i].APIKey == match {
					targetIndex = i
					break
				}
			}
		}
	}
	if targetIndex == -1 {
		c.JSON(404, gin.H{"error": "item not found"})
		return
	}

	entry := h.cfg.GeminiKey[targetIndex]
	if body.Value.APIKey != nil {
		trimmed := strings.TrimSpace(*body.Value.APIKey)
		if trimmed == "" {
			h.cfg.GeminiKey = append(h.cfg.GeminiKey[:targetIndex], h.cfg.GeminiKey[targetIndex+1:]...)
			h.cfg.SanitizeGeminiKeys()
			h.persist(c)
			return
		}
		entry.APIKey = trimmed
	}
	if body.Value.Prefix != nil {
		entry.Prefix = strings.TrimSpace(*body.Value.Prefix)
	}
	if body.Value.BaseURL != nil {
		entry.BaseURL = strings.TrimSpace(*body.Value.BaseURL)
	}
	if body.Value.ProxyURL != nil {
		entry.ProxyURL = strings.TrimSpace(*body.Value.ProxyURL)
	}
	if body.Value.Headers != nil {
		entry.Headers = config.NormalizeHeaders(*body.Value.Headers)
	}
	if body.Value.ExcludedModels != nil {
		entry.ExcludedModels = config.NormalizeExcludedModels(*body.Value.ExcludedModels)
	}
	h.cfg.GeminiKey[targetIndex] = entry
	h.cfg.SanitizeGeminiKeys()
	h.persist(c)
}

func (h *Handler) DeleteGeminiKey(c *gin.Context) {
	if val := strings.TrimSpace(c.Query("api-key")); val != "" {
		out := make([]config.GeminiKey, 0, len(h.cfg.GeminiKey))
		for _, v := range h.cfg.GeminiKey {
			if v.APIKey != val {
				out = append(out, v)
			}
		}
		if len(out) != len(h.cfg.GeminiKey) {
			h.cfg.GeminiKey = out
			h.cfg.SanitizeGeminiKeys()
			h.persist(c)
		} else {
			c.JSON(404, gin.H{"error": "item not found"})
		}
		return
	}
	if idxStr := c.Query("index"); idxStr != "" {
		var idx int
		if _, err := fmt.Sscanf(idxStr, "%d", &idx); err == nil && idx >= 0 && idx < len(h.cfg.GeminiKey) {
			h.cfg.GeminiKey = append(h.cfg.GeminiKey[:idx], h.cfg.GeminiKey[idx+1:]...)
			h.cfg.SanitizeGeminiKeys()
			h.persist(c)
			return
		}
	}
	c.JSON(400, gin.H{"error": "missing api-key or index"})
}

// claude-api-key: []ClaudeKey
func (h *Handler) GetClaudeKeys(c *gin.Context) {
	c.JSON(200, gin.H{"claude-api-key": h.cfg.ClaudeKey})
}
func (h *Handler) PutClaudeKeys(c *gin.Context) {
	data, err := c.GetRawData()
	if err != nil {
		c.JSON(400, gin.H{"error": "failed to read body"})
		return
	}
	var arr []config.ClaudeKey
	if err = json.Unmarshal(data, &arr); err != nil {
		var obj struct {
			Items []config.ClaudeKey `json:"items"`
		}
		if err2 := json.Unmarshal(data, &obj); err2 != nil || len(obj.Items) == 0 {
			c.JSON(400, gin.H{"error": "invalid body"})
			return
		}
		arr = obj.Items
	}
	for i := range arr {
		normalizeClaudeKey(&arr[i])
	}
	h.cfg.ClaudeKey = arr
	h.cfg.SanitizeClaudeKeys()
	h.persist(c)
}
func (h *Handler) PatchClaudeKey(c *gin.Context) {
	type claudeKeyPatch struct {
		APIKey         *string               `json:"api-key"`
		Prefix         *string               `json:"prefix"`
		BaseURL        *string               `json:"base-url"`
		ProxyURL       *string               `json:"proxy-url"`
		Models         *[]config.ClaudeModel `json:"models"`
		Headers        *map[string]string    `json:"headers"`
		ExcludedModels *[]string             `json:"excluded-models"`
	}
	var body struct {
		Index *int            `json:"index"`
		Match *string         `json:"match"`
		Value *claudeKeyPatch `json:"value"`
	}
	if err := c.ShouldBindJSON(&body); err != nil || body.Value == nil {
		c.JSON(400, gin.H{"error": "invalid body"})
		return
	}
	targetIndex := -1
	if body.Index != nil && *body.Index >= 0 && *body.Index < len(h.cfg.ClaudeKey) {
		targetIndex = *body.Index
	}
	if targetIndex == -1 && body.Match != nil {
		match := strings.TrimSpace(*body.Match)
		for i := range h.cfg.ClaudeKey {
			if h.cfg.ClaudeKey[i].APIKey == match {
				targetIndex = i
				break
			}
		}
	}
	if targetIndex == -1 {
		c.JSON(404, gin.H{"error": "item not found"})
		return
	}

	entry := h.cfg.ClaudeKey[targetIndex]
	if body.Value.APIKey != nil {
		entry.APIKey = strings.TrimSpace(*body.Value.APIKey)
	}
	if body.Value.Prefix != nil {
		entry.Prefix = strings.TrimSpace(*body.Value.Prefix)
	}
	if body.Value.BaseURL != nil {
		entry.BaseURL = strings.TrimSpace(*body.Value.BaseURL)
	}
	if body.Value.ProxyURL != nil {
		entry.ProxyURL = strings.TrimSpace(*body.Value.ProxyURL)
	}
	if body.Value.Models != nil {
		entry.Models = append([]config.ClaudeModel(nil), (*body.Value.Models)...)
	}
	if body.Value.Headers != nil {
		entry.Headers = config.NormalizeHeaders(*body.Value.Headers)
	}
	if body.Value.ExcludedModels != nil {
		entry.ExcludedModels = config.NormalizeExcludedModels(*body.Value.ExcludedModels)
	}
	normalizeClaudeKey(&entry)
	h.cfg.ClaudeKey[targetIndex] = entry
	h.cfg.SanitizeClaudeKeys()
	h.persist(c)
}

func (h *Handler) DeleteClaudeKey(c *gin.Context) {
	if val := c.Query("api-key"); val != "" {
		out := make([]config.ClaudeKey, 0, len(h.cfg.ClaudeKey))
		for _, v := range h.cfg.ClaudeKey {
			if v.APIKey != val {
				out = append(out, v)
			}
		}
		h.cfg.ClaudeKey = out
		h.cfg.SanitizeClaudeKeys()
		h.persist(c)
		return
	}
	if idxStr := c.Query("index"); idxStr != "" {
		var idx int
		_, err := fmt.Sscanf(idxStr, "%d", &idx)
		if err == nil && idx >= 0 && idx < len(h.cfg.ClaudeKey) {
			h.cfg.ClaudeKey = append(h.cfg.ClaudeKey[:idx], h.cfg.ClaudeKey[idx+1:]...)
			h.cfg.SanitizeClaudeKeys()
			h.persist(c)
			return
		}
	}
	c.JSON(400, gin.H{"error": "missing api-key or index"})
}

// openai-compatibility: []OpenAICompatibility
func (h *Handler) GetOpenAICompat(c *gin.Context) {
	c.JSON(200, gin.H{"openai-compatibility": normalizedOpenAICompatibilityEntries(h.cfg.OpenAICompatibility)})
}
func (h *Handler) PutOpenAICompat(c *gin.Context) {
	data, err := c.GetRawData()
	if err != nil {
		c.JSON(400, gin.H{"error": "failed to read body"})
		return
	}
	var arr []config.OpenAICompatibility
	if err = json.Unmarshal(data, &arr); err != nil {
		var obj struct {
			Items []config.OpenAICompatibility `json:"items"`
		}
		if err2 := json.Unmarshal(data, &obj); err2 != nil || len(obj.Items) == 0 {
			c.JSON(400, gin.H{"error": "invalid body"})
			return
		}
		arr = obj.Items
	}
	filtered := make([]config.OpenAICompatibility, 0, len(arr))
	for i := range arr {
		normalizeOpenAICompatibilityEntry(&arr[i])
		if strings.TrimSpace(arr[i].BaseURL) != "" {
			filtered = append(filtered, arr[i])
		}
	}
	h.cfg.OpenAICompatibility = filtered
	h.cfg.SanitizeOpenAICompatibility()
	h.persist(c)
}
func (h *Handler) PatchOpenAICompat(c *gin.Context) {
	type openAICompatPatch struct {
		Name          *string                             `json:"name"`
		Prefix        *string                             `json:"prefix"`
		BaseURL       *string                             `json:"base-url"`
		APIKeyEntries *[]config.OpenAICompatibilityAPIKey `json:"api-key-entries"`
		Models        *[]config.OpenAICompatibilityModel  `json:"models"`
		Headers       *map[string]string                  `json:"headers"`
	}
	var body struct {
		Name  *string            `json:"name"`
		Index *int               `json:"index"`
		Value *openAICompatPatch `json:"value"`
	}
	if err := c.ShouldBindJSON(&body); err != nil || body.Value == nil {
		c.JSON(400, gin.H{"error": "invalid body"})
		return
	}
	targetIndex := -1
	if body.Index != nil && *body.Index >= 0 && *body.Index < len(h.cfg.OpenAICompatibility) {
		targetIndex = *body.Index
	}
	if targetIndex == -1 && body.Name != nil {
		match := strings.TrimSpace(*body.Name)
		for i := range h.cfg.OpenAICompatibility {
			if h.cfg.OpenAICompatibility[i].Name == match {
				targetIndex = i
				break
			}
		}
	}
	if targetIndex == -1 {
		c.JSON(404, gin.H{"error": "item not found"})
		return
	}

	entry := h.cfg.OpenAICompatibility[targetIndex]
	if body.Value.Name != nil {
		entry.Name = strings.TrimSpace(*body.Value.Name)
	}
	if body.Value.Prefix != nil {
		entry.Prefix = strings.TrimSpace(*body.Value.Prefix)
	}
	if body.Value.BaseURL != nil {
		trimmed := strings.TrimSpace(*body.Value.BaseURL)
		if trimmed == "" {
			h.cfg.OpenAICompatibility = append(h.cfg.OpenAICompatibility[:targetIndex], h.cfg.OpenAICompatibility[targetIndex+1:]...)
			h.cfg.SanitizeOpenAICompatibility()
			h.persist(c)
			return
		}
		entry.BaseURL = trimmed
	}
	if body.Value.APIKeyEntries != nil {
		entry.APIKeyEntries = append([]config.OpenAICompatibilityAPIKey(nil), (*body.Value.APIKeyEntries)...)
	}
	if body.Value.Models != nil {
		entry.Models = append([]config.OpenAICompatibilityModel(nil), (*body.Value.Models)...)
	}
	if body.Value.Headers != nil {
		entry.Headers = config.NormalizeHeaders(*body.Value.Headers)
	}
	normalizeOpenAICompatibilityEntry(&entry)
	h.cfg.OpenAICompatibility[targetIndex] = entry
	h.cfg.SanitizeOpenAICompatibility()
	h.persist(c)
}

func (h *Handler) DeleteOpenAICompat(c *gin.Context) {
	if name := c.Query("name"); name != "" {
		out := make([]config.OpenAICompatibility, 0, len(h.cfg.OpenAICompatibility))
		for _, v := range h.cfg.OpenAICompatibility {
			if v.Name != name {
				out = append(out, v)
			}
		}
		h.cfg.OpenAICompatibility = out
		h.cfg.SanitizeOpenAICompatibility()
		h.persist(c)
		return
	}
	if idxStr := c.Query("index"); idxStr != "" {
		var idx int
		_, err := fmt.Sscanf(idxStr, "%d", &idx)
		if err == nil && idx >= 0 && idx < len(h.cfg.OpenAICompatibility) {
			h.cfg.OpenAICompatibility = append(h.cfg.OpenAICompatibility[:idx], h.cfg.OpenAICompatibility[idx+1:]...)
			h.cfg.SanitizeOpenAICompatibility()
			h.persist(c)
			return
		}
	}
	c.JSON(400, gin.H{"error": "missing name or index"})
}

// vertex-api-key: []VertexCompatKey
func (h *Handler) GetVertexCompatKeys(c *gin.Context) {
	c.JSON(200, gin.H{"vertex-api-key": h.cfg.VertexCompatAPIKey})
}
func (h *Handler) PutVertexCompatKeys(c *gin.Context) {
	data, err := c.GetRawData()
	if err != nil {
		c.JSON(400, gin.H{"error": "failed to read body"})
		return
	}
	var arr []config.VertexCompatKey
	if err = json.Unmarshal(data, &arr); err != nil {
		var obj struct {
			Items []config.VertexCompatKey `json:"items"`
		}
		if err2 := json.Unmarshal(data, &obj); err2 != nil || len(obj.Items) == 0 {
			c.JSON(400, gin.H{"error": "invalid body"})
			return
		}
		arr = obj.Items
	}
	for i := range arr {
		normalizeVertexCompatKey(&arr[i])
	}
	h.cfg.VertexCompatAPIKey = arr
	h.cfg.SanitizeVertexCompatKeys()
	h.persist(c)
}
func (h *Handler) PatchVertexCompatKey(c *gin.Context) {
	type vertexCompatPatch struct {
		APIKey   *string                     `json:"api-key"`
		Prefix   *string                     `json:"prefix"`
		BaseURL  *string                     `json:"base-url"`
		ProxyURL *string                     `json:"proxy-url"`
		Headers  *map[string]string          `json:"headers"`
		Models   *[]config.VertexCompatModel `json:"models"`
	}
	var body struct {
		Index *int               `json:"index"`
		Match *string            `json:"match"`
		Value *vertexCompatPatch `json:"value"`
	}
	if errBindJSON := c.ShouldBindJSON(&body); errBindJSON != nil || body.Value == nil {
		c.JSON(400, gin.H{"error": "invalid body"})
		return
	}
	targetIndex := -1
	if body.Index != nil && *body.Index >= 0 && *body.Index < len(h.cfg.VertexCompatAPIKey) {
		targetIndex = *body.Index
	}
	if targetIndex == -1 && body.Match != nil {
		match := strings.TrimSpace(*body.Match)
		if match != "" {
			for i := range h.cfg.VertexCompatAPIKey {
				if h.cfg.VertexCompatAPIKey[i].APIKey == match {
					targetIndex = i
					break
				}
			}
		}
	}
	if targetIndex == -1 {
		c.JSON(404, gin.H{"error": "item not found"})
		return
	}

	entry := h.cfg.VertexCompatAPIKey[targetIndex]
	if body.Value.APIKey != nil {
		trimmed := strings.TrimSpace(*body.Value.APIKey)
		if trimmed == "" {
			h.cfg.VertexCompatAPIKey = append(h.cfg.VertexCompatAPIKey[:targetIndex], h.cfg.VertexCompatAPIKey[targetIndex+1:]...)
			h.cfg.SanitizeVertexCompatKeys()
			h.persist(c)
			return
		}
		entry.APIKey = trimmed
	}
	if body.Value.Prefix != nil {
		entry.Prefix = strings.TrimSpace(*body.Value.Prefix)
	}
	if body.Value.BaseURL != nil {
		trimmed := strings.TrimSpace(*body.Value.BaseURL)
		if trimmed == "" {
			h.cfg.VertexCompatAPIKey = append(h.cfg.VertexCompatAPIKey[:targetIndex], h.cfg.VertexCompatAPIKey[targetIndex+1:]...)
			h.cfg.SanitizeVertexCompatKeys()
			h.persist(c)
			return
		}
		entry.BaseURL = trimmed
	}
	if body.Value.ProxyURL != nil {
		entry.ProxyURL = strings.TrimSpace(*body.Value.ProxyURL)
	}
	if body.Value.Headers != nil {
		entry.Headers = config.NormalizeHeaders(*body.Value.Headers)
	}
	if body.Value.Models != nil {
		entry.Models = append([]config.VertexCompatModel(nil), (*body.Value.Models)...)
	}
	normalizeVertexCompatKey(&entry)
	h.cfg.VertexCompatAPIKey[targetIndex] = entry
	h.cfg.SanitizeVertexCompatKeys()
	h.persist(c)
}

func (h *Handler) DeleteVertexCompatKey(c *gin.Context) {
	if val := strings.TrimSpace(c.Query("api-key")); val != "" {
		out := make([]config.VertexCompatKey, 0, len(h.cfg.VertexCompatAPIKey))
		for _, v := range h.cfg.VertexCompatAPIKey {
			if v.APIKey != val {
				out = append(out, v)
			}
		}
		h.cfg.VertexCompatAPIKey = out
		h.cfg.SanitizeVertexCompatKeys()
		h.persist(c)
		return
	}
	if idxStr := c.Query("index"); idxStr != "" {
		var idx int
		_, errScan := fmt.Sscanf(idxStr, "%d", &idx)
		if errScan == nil && idx >= 0 && idx < len(h.cfg.VertexCompatAPIKey) {
			h.cfg.VertexCompatAPIKey = append(h.cfg.VertexCompatAPIKey[:idx], h.cfg.VertexCompatAPIKey[idx+1:]...)
			h.cfg.SanitizeVertexCompatKeys()
			h.persist(c)
			return
		}
	}
	c.JSON(400, gin.H{"error": "missing api-key or index"})
}

// codex-api-key: []CodexKey
func (h *Handler) GetCodexKeys(c *gin.Context) {
	c.JSON(200, gin.H{"codex-api-key": h.cfg.CodexKey})
}
func (h *Handler) PutCodexKeys(c *gin.Context) {
	data, err := c.GetRawData()
	if err != nil {
		c.JSON(400, gin.H{"error": "failed to read body"})
		return
	}
	var arr []config.CodexKey
	if err = json.Unmarshal(data, &arr); err != nil {
		var obj struct {
			Items []config.CodexKey `json:"items"`
		}
		if err2 := json.Unmarshal(data, &obj); err2 != nil || len(obj.Items) == 0 {
			c.JSON(400, gin.H{"error": "invalid body"})
			return
		}
		arr = obj.Items
	}
	// Filter out codex entries with empty base-url (treat as removed)
	filtered := make([]config.CodexKey, 0, len(arr))
	for i := range arr {
		entry := arr[i]
		normalizeCodexKey(&entry)
		if entry.BaseURL == "" {
			continue
		}
		filtered = append(filtered, entry)
	}
	h.cfg.CodexKey = filtered
	h.cfg.SanitizeCodexKeys()
	h.persist(c)
}
func (h *Handler) PatchCodexKey(c *gin.Context) {
	type codexKeyPatch struct {
		APIKey         *string              `json:"api-key"`
		Prefix         *string              `json:"prefix"`
		BaseURL        *string              `json:"base-url"`
		ProxyURL       *string              `json:"proxy-url"`
		Models         *[]config.CodexModel `json:"models"`
		Headers        *map[string]string   `json:"headers"`
		ExcludedModels *[]string            `json:"excluded-models"`
	}
	var body struct {
		Index *int           `json:"index"`
		Match *string        `json:"match"`
		Value *codexKeyPatch `json:"value"`
	}
	if err := c.ShouldBindJSON(&body); err != nil || body.Value == nil {
		c.JSON(400, gin.H{"error": "invalid body"})
		return
	}
	targetIndex := -1
	if body.Index != nil && *body.Index >= 0 && *body.Index < len(h.cfg.CodexKey) {
		targetIndex = *body.Index
	}
	if targetIndex == -1 && body.Match != nil {
		match := strings.TrimSpace(*body.Match)
		for i := range h.cfg.CodexKey {
			if h.cfg.CodexKey[i].APIKey == match {
				targetIndex = i
				break
			}
		}
	}
	if targetIndex == -1 {
		c.JSON(404, gin.H{"error": "item not found"})
		return
	}

	entry := h.cfg.CodexKey[targetIndex]
	if body.Value.APIKey != nil {
		entry.APIKey = strings.TrimSpace(*body.Value.APIKey)
	}
	if body.Value.Prefix != nil {
		entry.Prefix = strings.TrimSpace(*body.Value.Prefix)
	}
	if body.Value.BaseURL != nil {
		trimmed := strings.TrimSpace(*body.Value.BaseURL)
		if trimmed == "" {
			h.cfg.CodexKey = append(h.cfg.CodexKey[:targetIndex], h.cfg.CodexKey[targetIndex+1:]...)
			h.cfg.SanitizeCodexKeys()
			h.persist(c)
			return
		}
		entry.BaseURL = trimmed
	}
	if body.Value.ProxyURL != nil {
		entry.ProxyURL = strings.TrimSpace(*body.Value.ProxyURL)
	}
	if body.Value.Models != nil {
		entry.Models = append([]config.CodexModel(nil), (*body.Value.Models)...)
	}
	if body.Value.Headers != nil {
		entry.Headers = config.NormalizeHeaders(*body.Value.Headers)
	}
	if body.Value.ExcludedModels != nil {
		entry.ExcludedModels = config.NormalizeExcludedModels(*body.Value.ExcludedModels)
	}
	normalizeCodexKey(&entry)
	h.cfg.CodexKey[targetIndex] = entry
	h.cfg.SanitizeCodexKeys()
	h.persist(c)
}

func (h *Handler) DeleteCodexKey(c *gin.Context) {
	if val := c.Query("api-key"); val != "" {
		out := make([]config.CodexKey, 0, len(h.cfg.CodexKey))
		for _, v := range h.cfg.CodexKey {
			if v.APIKey != val {
				out = append(out, v)
			}
		}
		h.cfg.CodexKey = out
		h.cfg.SanitizeCodexKeys()
		h.persist(c)
		return
	}
	if idxStr := c.Query("index"); idxStr != "" {
		var idx int
		_, err := fmt.Sscanf(idxStr, "%d", &idx)
		if err == nil && idx >= 0 && idx < len(h.cfg.CodexKey) {
			h.cfg.CodexKey = append(h.cfg.CodexKey[:idx], h.cfg.CodexKey[idx+1:]...)
			h.cfg.SanitizeCodexKeys()
			h.persist(c)
			return
		}
	}
	c.JSON(400, gin.H{"error": "missing api-key or index"})
}
