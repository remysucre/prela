
WITH UserVoteSummary AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(v.Id) AS TotalVotes,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
    FROM 
        Users u
    LEFT JOIN 
        Votes v ON u.Id = v.UserId
    GROUP BY 
        u.Id, u.DisplayName
), RecentPostActivity AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.LastActivityDate,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVoteCount,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVoteCount,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.LastActivityDate DESC, p.Id) AS ActivityRank
    FROM 
        Posts p
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    WHERE 
        p.CreationDate >= CAST('2024-10-01' AS DATE) - INTERVAL '30 days'
    GROUP BY 
        p.Id, p.Title, p.CreationDate, p.LastActivityDate, p.OwnerUserId
), ClosedPosts AS (
    SELECT 
        p.Id AS ClosedPostId,
        ph.UserDisplayName,
        ph.CreationDate AS CloseDate,
        ph.Comment AS CloseReason 
    FROM 
        PostHistory ph
    JOIN 
        Posts p ON ph.PostId = p.Id
    WHERE 
        ph.PostHistoryTypeId = 10
)
SELECT 
    uvs.DisplayName AS UserName,
    uvs.TotalVotes,
    uvs.UpVotes,
    uvs.DownVotes,
    rpa.PostId,
    rpa.Title AS RecentPostTitle,
    rpa.CreationDate AS PostCreationDate,
    rpa.LastActivityDate AS PostLastActivity,
    rpa.UpVoteCount,
    rpa.DownVoteCount,
    cp.ClosedPostId,
    cp.CloseDate,
    cp.CloseReason
FROM 
    UserVoteSummary uvs
JOIN 
    RecentPostActivity rpa ON rpa.ActivityRank = 1 
LEFT JOIN 
    ClosedPosts cp ON rpa.PostId = cp.ClosedPostId
WHERE 
    rpa.UpVoteCount > 3 AND uvs.TotalVotes > 10
ORDER BY 
    uvs.TotalVotes DESC, rpa.LastActivityDate DESC;
