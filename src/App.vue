<template>
  <div id="app">
    <div>
      <b-card no-body>
        <b-tabs card>
          <b-tab active>
            <template v-slot:title>
              1. Add Lightframes
              <b-badge variant="light" v-if="$refs.lightframes.loading_exif === false">{{ $refs.lightframes.numImages }}</b-badge>
              <b-spinner type="border" small v-if="$refs.lightframes.loading_exif === true"></b-spinner>
            </template>
            <b-card-text><ImageSelection ref="lightframes"/></b-card-text>
          </b-tab>
          <b-tab disabled>
            <template v-slot:title>
              2. Add Darkframes
              <b-badge variant="light" v-if="$refs.darkframes.loading_exif === false">{{ $refs.darkframes.numImages }}</b-badge>
              <b-spinner type="border" small v-if="$refs.darkframes.loading_exif === true"></b-spinner>
            </template>
            <b-card-text><ImageSelection ref="darkframes"/></b-card-text>
          </b-tab>
          <b-tab title="3. Process images">
            <b-card-text><ManageProcessing @start-processing="run_processing" ref="settings"/></b-card-text>
          </b-tab>
        </b-tabs>
      </b-card>
    </div>
  </div>
</template>

<script>
import ImageSelection from "@/components/ImageSelection";
import ManageProcessing from "@/components/ManageProcessing";
import {promisified} from "tauri/api/tauri";


export default {
  name: 'App',
  components: {
    ImageSelection,
    ManageProcessing
  },
  methods: {
    run_processing: function () {
      let parent = this
      console.log('run', parent.$refs.settings.merge_mode, parent.$refs.settings.output_path)
      promisified({
        cmd: "runMerge",
        out_path: parent.$refs.settings.output_path,
        mode_str: parent.$refs.settings.merge_mode,
        lightframes: parent.$refs.lightframes.sortedImages.map(img => img.path)
      }).then(function () {
        console.log("Finished merge")
      })
    }
  }
}
</script>

<style lang="scss">
@import 'assets/darkly.scss';

// Bootstrap and its default variables
@import '../node_modules/bootstrap/scss/bootstrap';
// BootstrapVue and its default variables
@import '../node_modules/bootstrap-vue/src/index.scss';
</style>
