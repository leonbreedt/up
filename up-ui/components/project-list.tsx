import React from 'react';
import Project from '../models/project';
import useSWR from 'swr';
import fetcher from '../lib/fetcher';

function ProjectList() {
  const { data } = useSWR<Project[]>('/api/projects', fetcher);
  return (
    <>
      <table>
        <thead>
          <tr>
            <th>Name</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {data ? (
            data.map((project) => (
              <tr key={project.id}>
                <td>{project.name}</td>
                <td>
                  <a href={`/edit/${project.id}`}>Edit</a>
                </td>
              </tr>
            ))
          ) : (
            <tr>
              <td colSpan={2}>Loading...</td>
            </tr>
          )}
        </tbody>
      </table>
    </>
  );
}

export default ProjectList;
