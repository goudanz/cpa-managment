package management

import (
	"encoding/json"
	"fmt"
	"strings"

	"github.com/gin-gonic/gin"
	"github.com/router-for-me/CLIProxyAPI/v6/internal/config"
)

// api-keys
func (h *Handler) GetAPIKeys(c *gin.Context) { c.JSON(200, gin.H{"api-keys": h.cfg.APIKeys}) }
func (h *Handler) PutAPIKeys(c *gin.Context) {
	h.putStringList(c, func(v []string) {
		h.cfg.APIKeys = append([]string(nil), v...)
	}, nil)
}
func (h *Handler) PatchAPIKeys(c *gin.Context) {
	h.patchStringList(c, &h.cfg.APIKeys, func() {})
}
func (h *Handler) DeleteAPIKeys(c *gin.Context) {
	h.deleteFromStringList(c, &h.cfg.APIKeys, func() {})
}

// api-key-policies
func (h *Handler) GetAPIKeyPolicies(c *gin.Context) {
	c.JSON(200, gin.H{"api-key-policies": h.cfg.APIKeyPolicies})
}

func (h *Handler) PutAPIKeyPolicies(c *gin.Context) {
	data, err := c.GetRawData()
	if err != nil {
		c.JSON(400, gin.H{"error": "failed to read body"})
		return
	}
	var arr []config.APIKeyPolicy
	if err = json.Unmarshal(data, &arr); err != nil {
		var obj struct {
			Items []config.APIKeyPolicy `json:"items"`
		}
		if err2 := json.Unmarshal(data, &obj); err2 != nil {
			c.JSON(400, gin.H{"error": "invalid body"})
			return
		}
		arr = obj.Items
	}
	h.cfg.APIKeyPolicies = append([]config.APIKeyPolicy(nil), arr...)
	h.cfg.SanitizeAPIKeyPolicies()
	h.persist(c)
}

func (h *Handler) PatchAPIKeyPolicies(c *gin.Context) {
	var body struct {
		Index *int                 `json:"index"`
		Match *string              `json:"match"`
		Value *config.APIKeyPolicy `json:"value"`
	}
	if err := c.ShouldBindJSON(&body); err != nil || body.Value == nil {
		c.JSON(400, gin.H{"error": "invalid body"})
		return
	}

	target := -1
	if body.Index != nil && *body.Index >= 0 && *body.Index < len(h.cfg.APIKeyPolicies) {
		target = *body.Index
	}
	if target == -1 && body.Match != nil {
		match := strings.TrimSpace(*body.Match)
		for i := range h.cfg.APIKeyPolicies {
			if strings.TrimSpace(h.cfg.APIKeyPolicies[i].APIKey) == match {
				target = i
				break
			}
		}
	}

	if target == -1 {
		h.cfg.APIKeyPolicies = append(h.cfg.APIKeyPolicies, *body.Value)
	} else {
		h.cfg.APIKeyPolicies[target] = *body.Value
	}

	h.cfg.SanitizeAPIKeyPolicies()
	h.persist(c)
}

func (h *Handler) DeleteAPIKeyPolicies(c *gin.Context) {
	if idxStr := c.Query("index"); idxStr != "" {
		var idx int
		if _, err := fmt.Sscanf(idxStr, "%d", &idx); err == nil && idx >= 0 && idx < len(h.cfg.APIKeyPolicies) {
			h.cfg.APIKeyPolicies = append(h.cfg.APIKeyPolicies[:idx], h.cfg.APIKeyPolicies[idx+1:]...)
			h.cfg.SanitizeAPIKeyPolicies()
			h.persist(c)
			return
		}
	}

	if val := strings.TrimSpace(c.Query("api-key")); val != "" {
		out := make([]config.APIKeyPolicy, 0, len(h.cfg.APIKeyPolicies))
		for _, item := range h.cfg.APIKeyPolicies {
			if strings.TrimSpace(item.APIKey) != val {
				out = append(out, item)
			}
		}
		h.cfg.APIKeyPolicies = out
		h.cfg.SanitizeAPIKeyPolicies()
		h.persist(c)
		return
	}

	c.JSON(400, gin.H{"error": "missing index or api-key"})
}
