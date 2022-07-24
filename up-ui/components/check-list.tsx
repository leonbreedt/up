import React from 'react';
import Check from '../models/check';
import useSWR from 'swr';
import fetcher from '../lib/fetcher';

function CheckList() {
  const { data } = useSWR<Check[]>('/api/checks', fetcher);
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
            data.map((check) => (
              <tr key={check.id}>
                <td>{check.name}</td>
                <td>
                  <a href={`/checks/${check.id}`}>Edit</a>
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

export default CheckList;
