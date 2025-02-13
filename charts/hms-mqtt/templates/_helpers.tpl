{{- define "hms.secretName" -}}
{{- if .Values.existingSecret -}}
	{{ .Values.existingSecret }}
{{- else -}}
	{{ .Release.Name }}-secret
{{- end -}}
{{- end -}}


