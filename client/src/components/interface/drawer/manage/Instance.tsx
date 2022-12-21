/* eslint-disable no-nested-ternary */
import * as React from 'react'
import {
  ListItem,
  Autocomplete,
  createFilterOptions,
  TextField,
  Checkbox,
  Typography,
  List,
  ListItemText,
  ListItemButton,
  capitalize,
  ListItemIcon,
} from '@mui/material'
import CheckBoxOutlineBlank from '@mui/icons-material/CheckBoxOutlineBlank'
import IndeterminateCheckBoxOutlined from '@mui/icons-material/IndeterminateCheckBoxOutlined'
import CheckBox from '@mui/icons-material/CheckBox'
import type { FeatureCollection } from 'geojson'

import { KojiResponse } from '@assets/types'
import { useStatic } from '@hooks/useStatic'
import { useShapes } from '@hooks/useShapes'
import { getData } from '@services/fetches'

const icon = <CheckBoxOutlineBlank fontSize="small" color="primary" />
const checkedIcon = <CheckBox fontSize="small" color="primary" />
const partialIcon = (
  <IndeterminateCheckBoxOutlined fontSize="small" color="primary" />
)
const filterOptions = createFilterOptions({
  matchFrom: 'any',
  stringify: (option: string) => option,
})

interface Props {
  endpoint: string
  stateKey: 'instances' | 'geofences'
}
export default function InstanceSelect({ endpoint, stateKey }: Props) {
  const selected = useStatic((s) => s.selected)
  const fences = useStatic((s) => ({
    instances: s.instances,
    geofences: s.geofences,
  }))
  const setStatic = useStatic((s) => s.setStatic)
  const setFromCollection = useShapes((s) => s.setters.setFromCollection)

  const [loading, setLoading] = React.useState(false)

  React.useEffect(() => {
    if (!Object.keys(fences[stateKey]).length) {
      setLoading(true)
      getData<KojiResponse>(endpoint)
        .then((resp) => {
          if (resp) {
            setStatic(
              stateKey,
              Object.fromEntries(
                resp.data.features
                  .filter((f) => f.properties?.name)
                  .map((f) => [
                    `${f.properties?.name}_${f.properties?.type}_${stateKey}`,
                    {
                      ...f,
                      id: `${f.properties?.name}_${f.properties?.type}_${stateKey}`,
                    },
                  ]),
              ),
            )
          }
          setLoading(false)
        })
        // eslint-disable-next-line no-console
        .catch((e) => console.error(e))
    }
  }, [])

  return (
    <ListItem>
      <Autocomplete
        value={selected.filter((s) => fences[stateKey][s])}
        size="small"
        onChange={(_e, newValue) => {
          // const added = newValue.filter((s) => !selected.includes(s))
          // const deleted = selected.filter((s) => !newValue.includes(s))
          const newCollection: FeatureCollection = {
            type: 'FeatureCollection',
            features: newValue.map((s) => fences[stateKey][s]),
          }
          setStatic('selected', newValue)
          setFromCollection(newCollection, stateKey)
        }}
        filterOptions={filterOptions}
        selectOnFocus
        clearOnBlur
        multiple
        loading={loading}
        handleHomeEndKeys
        disableCloseOnSelect
        fullWidth
        groupBy={(option) => fences[stateKey][option]?.properties?.type}
        // sx={{ width: '100%', mx: 'auto' }}
        options={Object.keys(fences[stateKey]).sort((a, b) =>
          fences[stateKey][a].properties?.type?.localeCompare(
            fences[stateKey][b].properties?.type,
          ),
        )}
        renderTags={(val) => (
          <Typography align="center">({val.length})</Typography>
        )}
        renderOption={(props, option, { selected: s }) => (
          <li {...props}>
            <Checkbox
              icon={icon}
              checkedIcon={checkedIcon}
              style={{ marginRight: 8 }}
              checked={s}
            />
            {option.split('_').slice(0, -2).join('_')}
          </li>
        )}
        renderInput={(params) => (
          <TextField label="Select Instance" {...params} />
        )}
        renderGroup={({ key, group, children }) => {
          const allValues = Array.isArray(children)
            ? [...selected, ...children.map((x) => x.key)] // vaguely hacky way to select all filtered results
            : Object.keys(fences[stateKey]).filter(
                (k) => fences[stateKey][k]?.type === group,
              )
          const allSelected = allValues.every((v) => selected.includes(v))
          const partialSelected =
            allSelected ||
            selected.some(
              (v) => fences[stateKey][v]?.properties?.type === group,
            )
          return group ? (
            <List key={key}>
              <ListItemButton
                onClick={() => {
                  try {
                    setStatic(
                      'selected',
                      allSelected || partialSelected
                        ? selected.filter(
                            (v) =>
                              !allValues.includes(v) ||
                              fences[stateKey][v]?.properties?.type !== group,
                          )
                        : [
                            ...allValues,
                            ...selected.filter(
                              (v) =>
                                fences[stateKey][v]?.properties?.type !== group,
                            ),
                          ],
                    )
                  } catch (e) {
                    // eslint-disable-next-line no-console
                    console.error(e)
                  }
                }}
              >
                <ListItemIcon>
                  {allSelected
                    ? checkedIcon
                    : partialSelected
                    ? partialIcon
                    : icon}
                </ListItemIcon>
                <ListItemText
                  primary={capitalize(
                    group
                      .split('_')
                      .map((x) => capitalize(x))
                      .join(' '),
                  )}
                />
              </ListItemButton>
              {children}
            </List>
          ) : null
        }}
      />
    </ListItem>
  )
}
