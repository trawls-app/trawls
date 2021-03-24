<template>
  <div>
    <form>
      <div class="form-group">
        <label for="out_path">Output path</label>
        <div class="input-group">
          <input class="form-control" type="text" :placeholder="output_path" id="out_path" readonly>
          <div class="input-group-append">
            <b-button v-on:click="choose_output" variant="primary">Choose output</b-button>
          </div>
        </div>

        <small id="out_path_help" class="form-text text-muted">Where the resulting DNG file should be saved.</small>
      </div>
      <div class="form-group form-check">
        <div class="form-check form-check-inline">
          <input class="form-check-input" type="radio" id="normal_mode" v-model="merge_mode" v-bind:value="'normal'">
          <label class="form-check-label" for="normal_mode">Normal mode</label>
        </div>
        <div class="form-check form-check-inline">
          <input class="form-check-input" type="radio" id="falling_mode" v-model="merge_mode" v-bind:value="'falling'">
          <label class="form-check-label" for="falling_mode">Falling comets (fade out)</label>
        </div>
        <div class="form-check form-check-inline">
          <input class="form-check-input" type="radio" id="raising_mode" v-model="merge_mode" v-bind:value="'raising'">
          <label class="form-check-label" for="raising_mode">Raising comets (fade in)</label>
        </div>
      </div>
      <b-button v-on:click="$emit('start-processing')" variant="success">Start processing</b-button>
    </form>

    <br>
    Loading lightframes
    <b-icon icon="check-circle" v-if="state.loading_done" variant="success"></b-icon>
    <b-icon icon="arrow-clockwise" animation="spin" v-if="state.loading_done === false"></b-icon>
    <b-progress class="mt-2" :max="state.count_lights" show-value>
      <b-progress-bar :value="state.count_loaded_lights" variant="success"></b-progress-bar>
      <b-progress-bar :value="state.count_loading_lights" animated></b-progress-bar>
    </b-progress>

    <br>
    Merging images
    <b-icon icon="check-circle" v-if="state.merging_done" variant="success"></b-icon>
    <b-icon icon="arrow-clockwise" animation="spin" v-if="state.merging_done === false"></b-icon>
    <b-progress class="mt-2" :max="state.count_lights - 1" show-value>
      <b-progress-bar :value="state.count_merged" variant="success"></b-progress-bar>
      <b-progress-bar :value="state.count_merging" animated></b-progress-bar>
    </b-progress>
  </div>
</template>

<script>
import { listen } from 'tauri/api/event'
import { save } from 'tauri/api/dialog'

let vue = undefined

export default {
  name: "ManageProcessing",
  emits: {
    'start-processing': null
  },
  data: function () {
    return {
      output_path: null,
      merge_mode: "normal",
      state: {},
    }
  },
  created() { vue = this; },
  methods: {
    choose_output: function () {
      let parent = this
      save({filter: "*.dng"}).then(function (res) {
        parent.output_path = res
      })
    },
    update_state: function (updated_state) {
      this.state = updated_state.payload
    }
  },
}

listen('state_change', payload => { vue.update_state(payload) })
</script>

<style scoped>

</style>