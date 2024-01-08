{{- define "collector.name" -}}
	{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "collector.chart" -}}
	{{- .Chart.Name -}}
{{- end -}}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "collector.fullname" -}}
	{{- if .Values.fullnameOverride -}}
		{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
	{{- else -}}
		{{- $name := default .Chart.Name .Values.nameOverride -}}
		{{- if (contains $name .Release.Name) -}}
			{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
		{{- else -}}
			{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
		{{- end -}}
	{{- end -}}
{{- end -}}

{{- define "collector.image" -}}
	{{- with .Values.image -}}
		{{- printf "%s/%s:%s" .registry .name (.tag | toString) -}}
	{{- end -}}
{{- end -}}

{{- define "collector.labels" -}}
app.kubernetes.io/name: {{ template "collector.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ template "collector.chart" . }}
{{- if .Values.extraLabels -}}
{{- toYaml .Values.extraLabels -}}
{{- end -}}
{{- end -}}

{{- define "collector.hass_secret_name" -}}
	{{- default (printf "%s-%s" (include "collector.fullname" .) "hass") .Values.collector.hass.secret.name_override | quote }}
{{- end -}}

{{- define "collector.influxdb_secret_name" -}}
	{{- default (printf "%s-%s" (include "collector.fullname" .) "influxdb") .Values.collector.influxdb.secret.name_override | quote }}
{{- end -}}

