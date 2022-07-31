<template>
    <span>
        <div class="d-flex flex-row border-bottom border-secondary bg-danger" v-if="source.error">
            <div class="mr-auto p-2">{{ source.filename }}</div>
            <div class="p-2 col-error">
                <b-icon icon="patch-exclamation"></b-icon>
                {{ source.error }}
            </div>
            <div class="p-2 text-center col-small">
                <b-icon class="clickable-icon" icon="x-circle" v-on:click="remove_image(source.path)"></b-icon>
            </div>
        </div>
        <div class="d-flex flex-row border-bottom border-secondary" v-else>
            <div class="mr-auto p-2">{{ source.filename }}</div>
            <div class="p-2 text-center col-medium" :class="get_color_class('exposure', source.exposure_seconds)">{{ source.exposure_seconds }}s</div>
            <div class="p-2 text-center col-medium" :class="{ 'bg-warning': Math.abs(source.interval) > interval_warning_threshold}" v-if="show_interval">{{ source.interval }}<span v-if="source.interval">s</span></div>
            <div class="p-2 text-center col-medium" :class="get_color_class('aperture', source.aperture)">f{{ source.aperture }}</div>
            <div class="p-2 text-center col-medium" :class="get_color_class('iso', source.iso)">{{ source.iso }}</div>
            <div class="p-2 text-center col-large">{{ source.creation_time}}</div>
            <div class="p-2 text-center col-small">
                <b-icon class="clickable-icon" icon="x-circle" v-on:click="remove_image(source.path)"></b-icon>
            </div>
        </div>
    </span>
    
</template>

<script>
export default {
  name: "FrameRow",
  props: ['index', 'source', 'setting_values', 'color_mapping', 'show_interval', 'interval_warning_threshold', 'remove_image'],
  methods: {
    get_color_class: function (setting, value) {
      if (this.setting_values[setting].size <= 1) return "bg-palette-0"

      return "bg-palette-" + this.color_mapping[setting][value]
    },
  }
}
</script>

<style scoped lang="scss">

.col-small {
  min-width: 2em;
  max-width: 2em;
  text-align: center;
}

.col-medium {
  min-width: 5.5em;
  max-width: 5.5em;
  text-align: center;
}

.col-large {
  min-width: 12em;
  max-width: 12em;
  text-align: center;
}

.col-error {
  min-width: 34em;
  max-width: 34em;
}

.clickable-icon {
  cursor: pointer;
}

.bg-palette-1 { background-color: indigo; }
.bg-palette-2 { background-color: pink; }
.bg-palette-3 { background-color: darkgreen; }
.bg-palette-4 { background-color: darkcyan; }
.bg-palette-5 { background-color: darkgoldenrod; }
.bg-palette-6 { background-color: darkblue; }
.bg-palette-7 { background-color: purple; }
.bg-palette-8 { background-color: darkkhaki; }

</style>